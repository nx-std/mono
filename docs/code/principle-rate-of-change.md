---
name: "principle-rate-of-change"
description: "Rate of Change — one lifetime per type; keep volatile policy out of stable mechanism and resolve run-constant facts once. Load when a type holds both configuration and runtime state, when deciding where a resolved fact is pinned, or when splitting a module edited on two schedules"
type: "principle"
scope: "global"
---

# Rate of Change (Group by Lifetime, Split What Changes Apart)

## Rule

Things that change at the same rate belong together. Things that change at different rates belong apart, even
when they are about the same subject. "Rate" is measured two ways: how often a **value** is replaced at
runtime, and how often a **line of code** is edited across releases.

1. **One lifetime per type.** Values fixed at startup, values refreshed periodically, and values that live for
   one request or one job are three lifetimes and belong in three types. A type that mixes them has to be
   reasoned about at the fastest rate it contains, and its slow fields acquire an `Option` or a lock they did
   not need.
2. **Resolve a fact at the rate the fact changes, once.** A decision that is fixed for a run is made once at
   the start of the run and carried as a value. Re-deriving it inside a loop means a change landing mid-run
   can be observed halfway through, and the same input stops producing the same output.
3. **Volatile policy does not live inside stable mechanism.** A decoder for a format that has not moved in a
   year, and rules that move every release, are edited by different people for different reasons. Keep the
   rules out of the decoder and pass it the decoded value.
4. **Rate decides what is cohesive.** This principle overrides topical grouping: two things about the same
   subject that change on different schedules are two concerns, and two near-identical things that change on
   different schedules were never one fact, whatever `principle-dry-wet` would say about the duplication.
5. **Use history as evidence, not intuition.** A file where half the lines were last touched this month and
   half two years ago is naming its own seam. That signal is real; "this might change one day" is not.

## Examples

1. **One lifetime per type**
   A service client holding its startup-resolved sizing, its process-lifetime kernel handles, and its
   per-command bookkeeping in one struct.

```rust
// ❌ Bad — three lifetimes in one type. The two per-command fields are meaningless
// between commands, so they are `Option`, and every method that touches them carries
// an unwrap or a "no command in flight" branch. Worse, the whole struct sits behind a
// `Mutex` because two of six fields mutate, so a caller that only wants to read the
// negotiated interface version blocks behind someone else's unrelated request.
pub struct GpuClient {
    tx_buf_len: usize,               // fixed at startup
    iface_version: IfaceVersion,     // fixed at startup
    session: SessionHandle,          // process lifetime
    shmem: SharedMemoryHandle,       // process lifetime
    in_flight: Option<CommandId>,    // one command
    words_written: usize,            // one command
}
```

```rust
// ✅ Good — three types, three lifetimes. `CommandCall` is built per request, so its
// fields are never absent and never need a lock; `GpuClient` is `Copy` and freely
// shared because nothing in it mutates; `GpuConfig` is resolved once at startup and
// never written again.
pub struct GpuConfig {
    pub tx_buf_len: usize,
    pub iface_version: IfaceVersion,
}

#[derive(Clone, Copy)]
pub struct GpuClient {
    pub session: SessionHandle,
    pub shmem: SharedMemoryHandle,
}

pub struct CommandCall {
    pub command_id: CommandId,
    pub words_written: usize,
}
```

2. **Resolve once, at the rate the fact changes**
   Which command id a session speaks is fixed once the interface version is negotiated at startup; the
   session handle is fixed for the process.

```rust
// ❌ Bad — the HOS version is re-read from the runtime environment for every frame,
// through a handle that outlives the whole submission. The version cache is filled
// lazily during startup, so a submission begun early sent its head under the
// pre-1.0 command id and its tail under the modern one; the peer answered both and
// the caller saw a truncated queue rather than an error.
pub fn submit_all(
    client: &GpuClient,
    mut frames: FrameQueue,
) -> Result<(), DispatchError> {
    while let Some(frame) = frames.pop() {
        let command_id = submit_command_id(env::hos_version());
        client.dispatch(command_id, frame)?;
    }
    Ok(())
}
```

```rust
// ✅ Good — the command id is resolved once where the session is opened and carried
// as a value; the client holds only what lives as long as the process. Every frame
// in the submission speaks the same revision of the interface, and the loop cannot
// observe a change it was not built for.
pub fn submit_all(
    client: &GpuClient,
    command_id: CommandId,
    mut frames: FrameQueue,
) -> Result<(), DispatchError> {
    while let Some(frame) = frames.pop() {
        client.dispatch(command_id, frame)?;
    }
    Ok(())
}
```

3. **Volatile policy out of stable mechanism**
   A CMIF response decoder and the rules deciding which returned objects this build is willing to keep.

```rust
// ❌ Bad — the per-service acceptance rules live inside the decoder, so every rule
// change edits the one function that must not break, and the decoder's tests grow a
// fixture per rule. Widening the deprecated-interface list shipped an out-of-bounds
// read on an empty object list, because the reviewer was reading the policy, not the
// word-level parsing.
pub fn decode_response(raw: &RawMessage) -> Result<Response, DecodeError> {
    let header = CmifHeader::decode(raw.header_words())?;
    let mut objects = Vec::new();
    for raw_obj in raw.out_objects() {
        let obj = ObjectId::decode(raw_obj)?;
        if obj.is_null() && header.command_id == CMD_OPEN_DISPLAY {
            continue;
        }
        if DEPRECATED_INTERFACES.contains(&obj.iface) {
            continue;
        }
        objects.push(obj);
    }
    Ok(Response { header, objects })
}
```

```rust
// ✅ Good — the decoder only decodes, and is edited when the CMIF wire layout moves,
// which it has not. The policy is a separate function over decoded values, edited
// whenever a service revision lands, with its own tests and no way to corrupt the
// parsing of a message.
pub fn decode_response(raw: &RawMessage) -> Result<Response, DecodeError> {
    let objects = raw
        .out_objects()
        .map(ObjectId::decode)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Response { header: CmifHeader::decode(raw.header_words())?, objects })
}

/// Drop returned objects whose interfaces this build does not implement.
pub fn retain_supported(resp: &mut Response) {
    resp.objects.retain(|obj| !obj.is_null() && !DEPRECATED_INTERFACES.contains(&obj.iface));
}
```

## Why It Matters

A type is only as easy to reason about as its fastest-changing field. Put one mutable counter beside seven
immutable settings and the whole type needs a lock, loses `Clone`, and can no longer be shared; every reader
of the settings now pays for a field they never touch. The `Option` fields are the visible symptom: a field
that is absent outside one phase is a lifetime that wanted its own type, and the phase invariant it should
have carried is instead re-checked at every use.

Re-deriving a slow-changing fact at a fast rate is the version that costs correctness rather than legibility. A
value that was constant for the whole of a loop on the developer's console is not constant on every console,
where a lazily filled version cache or a loader-supplied service override resolves mid-loop, and the failure is
a sequence of requests that is internally inconsistent rather than one that stops.

The code side is paid in review. When volatile rules sit inside stable mechanism, every routine policy edit
arrives as a diff against parsing code, and the reviewer either reads the mechanism again or waves the change
through. Separated, the same edit touches a file whose tests are about exactly that question.

## Pragmatism Caveat

Rates are estimates, and splitting on a predicted rate is the same mistake as extracting a premature
abstraction. Split when the two rates are **structural** (per-request against process lifetime, build-time
against runtime) or when history already shows them, not because a field looks like it might churn. A type
holding two settings that have never moved independently is one concern until proven otherwise.

Splitting also has a price, and it is threading. If separating a value means passing it through five layers
that have no interest in it, the split may cost more than the coupling it removes; keep them together and say
why in a comment at the type. The same applies where a lifetime split would buy nothing: a small struct built
and dropped in one function does not need its phases separated.

When you knowingly keep two rates together, write the reason at the declaration. An undocumented mix is always
wrong, because the next reader cannot tell a deliberate choice from a type that simply accreted.

## Checklist

Before committing code, verify:

- [ ] No type mixes startup-resolved values, process-lifetime handles, and per-request or per-command state
- [ ] No field is `Option` only because it is absent outside one phase
- [ ] Nothing is locked because a minority of its fields mutate
- [ ] Facts that are constant for a run — heap base and size, process handle, loader environment, service
      availability — are resolved once at startup and carried as values
- [ ] No loop re-derives a value that cannot legitimately change while the loop runs
- [ ] Rules that change per service revision are not edited inside code that decodes, parses, or marshals
- [ ] Values with different sources (build time, startup, per request) have different homes
- [ ] Any deliberate mixing of rates carries a comment saying why

## References

- [principle-single-responsibility](principle-single-responsibility.md) - Related: Two rates of change are two
  reasons to change, which is the same split arrived at from the other side
- [principle-dry-wet](principle-dry-wet.md) - Related: Two copies that change on different schedules are two
  facts, however alike they look
- [principle-symmetry](principle-symmetry.md) - Related: Divergence on different schedules is when to break a
  symmetric pair on purpose
- [principle-type-driven-design](principle-type-driven-design.md) - Related: A per-phase type removes the
  `Option` fields that a mixed lifetime forces
- [principle-information-hiding](principle-information-hiding.md) - Related: Splitting by rate is what lets
  the volatile half change without widening the stable half's surface

## External References

- [Rate of Change, in Kent Beck's Implementation Patterns](https://zxuanhong.medium.com/kent-beck-implementation-pattern-principles-6-rate-of-change-4c63354cc84)
- [Tune Software Development for Rate of Change — Kent Beck](https://medium.com/@kentbeck_7670/tune-software-development-for-rate-of-change-not-rate-of-progress-56f93c15a769)
- [Shearing layers](https://en.wikipedia.org/wiki/Shearing_layers)
- [On the Criteria To Be Used in Decomposing Systems into Modules — Parnas](https://dl.acm.org/doi/10.1145/361598.361623)
