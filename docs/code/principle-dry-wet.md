---
name: "principle-dry-wet"
description: "DRY vs WET — deduplicate knowledge, tolerate coincidental similarity. Load when extracting shared helpers, creating abstractions, or reviewing duplicated-looking code"
type: "principle"
scope: "global"
---

# DRY/WET Balance (Don't Repeat Yourself vs. Write Everything Twice)

## Rule

Every piece of **knowledge** — a formula, a wire format, a register layout, a policy — has exactly one
authoritative representation. Deduplicate knowledge. Do **not** deduplicate code that merely looks alike but
belongs to independent concerns that will diverge.

Before extracting a shared helper, apply these checks:

1. **Same knowledge, not same shape**: does the duplication encode the same fact? The CMIF in-header layout is
   one fact about the IPC wire format. Two service clients that each send one request are two facts that
   happen to look alike.
2. **Rule of Three**: resist extracting on the second occurrence. Wait for the third, when you can see which
   parts actually vary.
3. **Inline test for a wrong abstraction**: if the shared function has a parameter or conditional whose only
   job is to pick a caller's behavior, the abstraction is wrong. Inline it back and let each caller evolve.

**Duplication is far cheaper than the wrong abstraction.** Inlining a premature abstraction is progress.

## Examples

1. **Same knowledge — one authoritative representation**
   The CMIF in-header layout is a fact about the wire format; every service client that builds a request
   needs it.

```rust
// ❌ Bad — the header-packing rule is re-derived in each service crate. Adding the
// context token to the header means finding every hand-packed word, and missing one
// leaves that service sending a request the sysmodule rejects with 0xF601 at runtime,
// on hardware, with no compile error to point at the crate that got it wrong.
let words = [
    SFCI_MAGIC,
    CMIF_VERSION,
    request_id,
    0, // token
];
// ...and again in the settings client
let words = [SFCI_MAGIC, CMIF_VERSION, cmd, 0];
// ...and again in the fs client, this time packing the version into the high half-word
let hdr = (SFCI_MAGIC as u64) | ((CMIF_VERSION as u64) << 32);
```

```rust
// ✅ Good — one type in nx-sf owns the fact; every service client asks for it. Adding
// the context token becomes a one-line change to a single struct instead of a sweep
// across dozens of nx-service-* crates.
/// In-band CMIF request header, exactly as it appears in the data-words region.
#[repr(C)]
#[derive(zerocopy::IntoBytes, zerocopy::Immutable)]
pub struct InHeader {
    magic: U32<LittleEndian>,
    version: U32<LittleEndian>,
    request_id: U32<LittleEndian>,
    token: U32<LittleEndian>,
}
```

2. **Coincidental similarity — keep them separate**
   Two service clients, structurally identical today, encoding different service contracts.

```rust
// ❌ Bad — one shared entry point, because "both clients just send one request".
// `service` and `request_id` are not shared knowledge; they are each service's own
// contract. The first command that needs an output buffer, a copied handle, or a
// domain object gets bolted on as another parameter, and every service client
// inherits the widened signature — plus the panic path for arguments its own
// service can never produce.
pub fn send_simple(session: SessionHandle, request_id: u32) -> Result<(), SendError> {
    // one body, N services' assumptions
}
```

```rust
// ✅ Good — two functions, two service contracts, free to diverge. The bodies look
// alike; the knowledge does not. Note this stays hand-written: a `macro_rules!`
// generator would collapse both into one expansion site and hide exactly the
// per-service differences (error type, buffer mode, request id) that the next
// command is going to introduce.
/// Clears the pending fatal report. Takes no arguments and returns nothing.
pub fn clear_report(session: SessionHandle) -> Result<(), ClearReportError> {
    // ...
}

/// Cancels the in-flight clock adjustment. Takes no arguments and returns nothing.
pub fn cancel_adjustment(session: SessionHandle) -> Result<(), CancelAdjustmentError> {
    // ...
}
```

3. **Wrong abstraction — inline it back**
   One builder serializes a **CMIF** request, another a **TIPC** request. They look almost identical. They are
   not the same knowledge.

```rust
// ❌ Bad — one "universal" builder with a protocol flag. The two protocols already differ
// (CMIF carries the SFCI in-header, a version word and a context token, and pads the data
// words to 16 bytes; TIPC has no in-header at all and encodes the request id in the HIPC
// command type). Every protocol change adds another flag, and each flag is a chance to
// break the other caller on hardware only.
fn build_request(
    request_id: u32,
    proto: Protocol,
    options: BuildOptions, // { emit_in_header, align_data_words, request_id_in_command_type }
) -> Result<(), WriteError> {
    // ~80 lines of `if proto == Protocol::Cmif { ... } else { ... }`
}
```

```rust
// ✅ Good — two builders, each owning one protocol's rules, each independently testable.
// A change to CMIF's in-header cannot reach the TIPC builder or its tests.
/// Build a CMIF request: SFCI in-header, version, context token, 16-byte-aligned words.
pub fn build_cmif(request_id: u32, payload: &[u8]) -> Result<CmifRequest<'_>, WriteError> {
    // ...
}

/// Build a TIPC request: no in-band header; the request id rides in the HIPC command type.
pub fn build_tipc(request_id: u32, payload: &[u8]) -> Result<TipcRequest<'_>, WriteError> {
    // ...
}
```

## Why It Matters

Duplicated knowledge means coordinated edits. Miss one copy of a header layout and that one service client
sends a malformed request the kernel happily delivers — a failure that shows up as a `ResultCode` on a console,
not as a compile error and not as a test failure on the host.

Duplicated _shape_ forced into one abstraction costs more. A shared `build_request(id, proto, options)` couples
CMIF to TIPC: a change on one side touches code the other side's tests cover, and the flags grow until nobody
can say what the function does without reading every branch. Undoing that is harder than never building it.

## Pragmatism Caveat

The Rule of Three is a heuristic. Two occurrences of an unmistakable fact (a wire-format constant, a magic
number from the IPC spec, an SVC number) can be extracted immediately; three occurrences that serve three
protocols should stay apart.

Small helpers duplicated across module or crate boundaries are usually correct. A four-line result-code
conversion copied into two sibling service crates is not a violation: promoting it to `pub(crate)` or hoisting
it into `nx-sf` to save eight lines widens an API surface and stops the two crates changing independently.
Prefer the private copy. The same reasoning rules out a `macro_rules!` written purely to collapse repeated
forwarding methods: the macro removes the lines but also removes the place where a per-caller difference would
be visible.

When you keep duplication on purpose, say so in a comment. When you extract, make sure the name describes the
shared _concept_ (`InHeader`), not the shared _shape_ (`build_request`, `send_simple`). An undocumented
decision either way is always wrong.

## Checklist

Before committing code, verify:

- [ ] Extracted code encodes one fact, not one syntax shape
- [ ] No shared helper takes a flag, mode, or `kind` parameter that exists only to select a caller's behavior
- [ ] Wire-format constants, header and register layouts, and spec values have exactly one definition
- [ ] Similar-looking code that serves two protocols or two service contracts stays in two places
- [ ] Deliberate duplication carries a comment explaining that the similarity is coincidental
- [ ] Cross-crate hoisting is justified by shared knowledge, not by line count

## References

- [principle-single-responsibility](principle-single-responsibility.md) - Related: A helper serving two concerns
  is the wrong abstraction by definition
- [principle-open-closed](principle-open-closed.md) - Related: Registries and extension points are where
  genuinely shared behavior belongs; flags are not
- [principle-least-surprise](principle-least-surprise.md) - Related: An abstraction named for its shape rather
  than its concept surprises every caller
- [principle-symmetry](principle-symmetry.md) - Related: Make near-duplicates symmetric first; only then is it
  visible whether they are one fact or two
- [principle-rate-of-change](principle-rate-of-change.md) - Related: Two copies that change on different
  schedules are two facts, whatever their shape says

## External References

- [The Wrong Abstraction — Sandi Metz](https://sandimetz.com/blog/2016/1/20/the-wrong-abstraction)
- [DRY is about Knowledge (Verraes)](https://verraes.net/2014/08/dry-is-about-knowledge/)
- [Caught in a Bad Abstraction — Israeli Tech Radar](https://medium.com/israeli-tech-radar/caught-in-a-bad-abstraction-55bfe6634b83)
- [DRY: Most Over-rated Programming Principle — Gordon C](https://gordonc.bearblog.dev/dry-most-over-rated-programming-principle/)
