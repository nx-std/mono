---
name: "principle-symmetry"
description: "Symmetry — express the same idea the same way; split near-duplicates into identical parts and clearly different parts, one altitude per body. Load when writing something that resembles existing code, or reviewing sibling functions, branches, or modules"
type: "principle"
scope: "global"
---

# Symmetry (Express the Same Idea the Same Way)

## Rule

The same idea is expressed the same way everywhere it appears. When two pieces of code are **almost** the
same, split them so the parts that are identical are **literally identical** and the parts that differ are
the **only** visible difference. Symmetry is about form: it does not ask you to merge the two, it asks you to
make the difference legible.

1. **One idea, one shape.** Two functions that answer the same question take their parameters in the same
   order, return the same shape, and name their steps the same way. Ask "what does this do?" of each; the same
   answer from two different shapes is a violation. Which name to pick is settled by the `rust-fn` rule
   document; this document owns the case where two names, orders, or return shapes disagree with each other.
2. **Near-duplicates keep an identical skeleton.** Same step order, same local names, same error handling,
   with the divergence isolated to the lines that must diverge. Reordering steps or renaming locals between
   two variants hides the real difference in noise.
3. **One level of abstraction per function.** Every statement in a body sits at the same altitude. A sequence
   that names intentions does not contain one statement that leaks the mechanism.
4. **Sibling branches carry comparable weight.** Match arms and `if`/`else` branches of one construct all
   delegate, or all inline. A three-line arm beside a thirty-line arm is a missing extraction, visible before
   the logic is read.
5. **Paired operations stay paired.** What is acquired is released, what is encoded is decoded, what is
   registered is deregistered — in the same module, at the same level, in the same vocabulary. The naming
   contract for the inverse is owned by `principle-least-surprise`; what this document requires is that the
   pair exists and sits together.
6. **Sibling modules in the same role share a layout.** Crates or modules that play the same part expose the
   same entry points in the same file positions, so knowing one is knowing all of them.

**Symmetry is not deduplication.** Two symmetric copies with one visible difference are a good outcome, and
often a better one than a single parameterized abstraction; whether to extract at all is decided by
`principle-dry-wet`. Make the pair symmetric first, then decide.

## Examples

1. **One idea, one shape**
   Three lookups that answer the same question about the service registry, written three ways.

```rust
// ❌ Bad — same question, three shapes: the registry moves between first and last
// parameter, one swallows the `ResultCode` into `None`, and "not registered" is
// spelled as a `None`, as an error variant, and as a nested `Option`. Every caller
// has to open the callee to learn which. A retry wrapper written against one of them
// treated "not registered" as success for the other two and handed the caller a
// zeroed handle, which faulted in the kernel on the first request.
pub fn registered_port(reg: &Registry, name: ServiceName) -> Result<PortHandle, RegistryError>;
pub fn get_active_session(name: ServiceName, reg: &Registry) -> Option<SessionHandle>;
pub fn fetch_domain_object(reg: &Registry, name: ServiceName) -> Result<Option<ObjectId>, RegistryError>;
```

```rust
// ✅ Good — one shape for one idea: registry first, subject second, absence is `None`
// and failure is an error. A caller who has used one has used all three, and a
// helper written over one composes with the others unchanged.
pub fn registered_port(reg: &Registry, name: ServiceName) -> Result<Option<PortHandle>, RegistryError>;
pub fn active_session(reg: &Registry, name: ServiceName) -> Result<Option<SessionHandle>, RegistryError>;
pub fn domain_object(reg: &Registry, name: ServiceName) -> Result<Option<ObjectId>, RegistryError>;
```

2. **Near-duplicates keep an identical skeleton**
   Two request writers, one per IPC protocol. They stay two functions; what changes is that their difference
   becomes visible.

```rust
// ❌ Bad — the same five steps in a different order under different local names, so
// the one real difference (which header the protocol prepends) is buried. A fix to the
// word-count/header ordering landed in the first and was missed in the second for two
// releases, so every TIPC request under-reported its payload length and the server
// replied `0xF601` on anything with more than four words; nobody spotted it in review
// because the pair could not be read side by side.
pub fn write_cmif_request(
    tls: &mut IpcBuffer,
    req: &Request,
) -> Result<(), MarshalError> {
    let payload = req.payload_words();
    let cmif = CmifHeader::new(req.command_id, payload.len());
    let mut cursor = tls.cursor();
    cursor.write_hipc_header(HipcHeader::for_cmif(payload.len()))?;
    cursor.write_cmif_header(cmif)?;
    cursor.write_words(payload)?;
    Ok(())
}

pub fn tipc_marshal(
    tls: &mut IpcBuffer,
    req: &Request,
) -> Result<(), MarshalError> {
    let mut cur = tls.cursor();
    let words = req.payload_words();
    cur.write_hipc_header(HipcHeader::for_tipc(req.command_id, words.len()))?;
    let decoded = pad_to_word(words);
    cur.write_words(&decoded)?;
    Ok(())
}
```

```rust
// ✅ Good — identical skeleton, identical local names, identical order. Exactly two
// lines differ, and they are the two that must: which HIPC header is written, and
// whether a CMIF header precedes the payload. A reviewer diffs the pair at a glance
// and a change to one is an obvious prompt to look at the other.
pub fn write_cmif_request(
    tls: &mut IpcBuffer,
    req: &Request,
) -> Result<(), MarshalError> {
    let payload = req.payload_words();
    let mut cursor = tls.cursor();
    cursor.write_hipc_header(HipcHeader::for_cmif(payload.len()))?;
    cursor.write_cmif_header(CmifHeader::new(req.command_id, payload.len()))?;
    cursor.write_words(payload)?;
    Ok(())
}

pub fn write_tipc_request(
    tls: &mut IpcBuffer,
    req: &Request,
) -> Result<(), MarshalError> {
    let payload = req.payload_words();
    let mut cursor = tls.cursor();
    cursor.write_hipc_header(HipcHeader::for_tipc(req.command_id, payload.len()))?;
    cursor.write_words(payload)?;
    Ok(())
}
```

3. **One altitude, comparable branches**
   A dispatch over session lifecycle events, where one arm names an intention and the other performs the
   mechanism.

```rust
// ❌ Bad — the reader has to change altitude mid-match: two arms state what happens,
// the third states how. The slot lock, the `close_handle` SVC and the counter update
// are invisible from the call site, so a second writer of this match copied the short
// arms and forgot the `close_handle`; the process leaked one kernel handle per closed
// session until it hit `LimitReached` and every later `connect_to_named_port` failed.
fn apply(&self, event: SessionEvent) -> Result<(), SessionError> {
    match event {
        SessionEvent::Opened(id) => self.mark_active(id)?,
        SessionEvent::Detached(id) => self.mark_detached(id)?,
        SessionEvent::Closed(id) => {
            let mut slots = self.slots.lock();
            let slot = slots.take(id).ok_or(SessionError::UnknownSession(id))?;
            // SAFETY: `slot` owns the handle and was just removed from the table,
            // so no other thread can observe it after this point.
            unsafe { raw::close_handle(slot.handle.to_raw()) };
            slots.mark_free(id);
            self.closed.fetch_add(1, Ordering::Relaxed);
        }
    }
    Ok(())
}
```

```rust
// ✅ Good — every arm states an intention and nothing else, so the match reads as a
// list of outcomes. The handle close lives with the other state transitions, where the
// next one written will find it.
fn apply(&self, event: SessionEvent) -> Result<(), SessionError> {
    match event {
        SessionEvent::Opened(id) => self.mark_active(id)?,
        SessionEvent::Detached(id) => self.mark_detached(id)?,
        SessionEvent::Closed(id) => self.mark_closed(id)?,
    }
    Ok(())
}
```

## Why It Matters

Asymmetry is paid for on every read. A reader who has understood one member of a pair should be able to skip
the other; when the pair disagrees in shape, they must read both in full and then diff them by hand to find
out whether the difference is meaningful. That cost is invisible in a diff and unbounded over a file's life.

It is paid for again in bugs. Divergent shapes defeat the reviewer's strongest tool, which is noticing that
two things that should match do not: a fix applied to one variant and missed in the other passes review
precisely because nothing looks out of place. Mixed altitude within one body hides effects from the call site,
and unbalanced branches hide a whole procedure inside what reads as a case label.

Symmetry also compounds. Once every service crate exposes the same entry points in the same positions, and
every newtype declares its conversions in the same order, a reader lands in an unfamiliar crate already
knowing where to look. That is the return on consistency, and it is only available if the consistency holds
everywhere.

## Pragmatism Caveat

**False symmetry is worse than asymmetry.** Two things that are genuinely different must not be bent into one
shape, because a matching shape is a claim that the behavior matches, and a reader will act on it. Do not pad
a branch to balance it, do not give an infallible function a `Result` so it lines up with its neighbor, and do
not invent a `close()` for a type that owns nothing.

Symmetry is also bounded by the seams around it. A trait from a dependency dictates its own parameter order
and return shape: match the foreign shape at the boundary and the workspace shape everywhere else. Where two
variants are diverging permanently, breaking their symmetry on purpose is the right call, and the cheap
version of it is renaming so the reader stops expecting a pair.

When you break symmetry deliberately, say so in a comment at the declaration. An undocumented asymmetry is
always wrong: the next reader cannot tell it from the copy nobody got around to updating.

## Checklist

Before committing code, verify:

- [ ] Functions answering the same question take the same parameter order and return the same shape
- [ ] Near-duplicate bodies share step order, local names, and error handling; only the intended lines differ
- [ ] No function body mixes statements that name an intention with statements that perform the mechanism
- [ ] Sibling match arms and branches all delegate or all inline; none hides a procedure
- [ ] Every acquire, encode, or register has its inverse in the same module at the same level
- [ ] Modules playing the same role expose the same entry points in the same positions
- [ ] No shape was matched that the behavior does not match, and no branch was padded to balance it
- [ ] Any deliberate asymmetry carries a comment saying why

## References

- [principle-dry-wet](principle-dry-wet.md) - Related: Symmetry makes the difference visible; DRY/WET decides
  whether the pair is one fact and should be extracted at all
- [principle-least-surprise](principle-least-surprise.md) - Related: Owns the naming contract for paired
  operations; a symmetric shape is what makes a name's prediction hold
- [principle-single-responsibility](principle-single-responsibility.md) - Related: A body that mixes altitudes
  is usually a function with two responsibilities
- [principle-open-closed](principle-open-closed.md) - Related: A registry stays extensible only while every
  entry has the same shape
- [principle-rate-of-change](principle-rate-of-change.md) - Related: Says when a symmetric pair should be
  broken on purpose, because the two halves have started moving on different schedules

## External References

- [Symmetry, in Kent Beck's Implementation Patterns](https://blog.iterate.no/2012/06/20/programming-like-kent-beck/)
- [Mastering Programming — Kent Beck](https://tidyfirst.substack.com/p/mastering-programming)
- [The Value of Symmetry — Scott Allen](https://odetocode.com/blogs/scott/archive/2011/02/07/the-value-of-symmetry.aspx)
- [Consistency creates cognitive leverage — A Philosophy of Software Design](https://danlebrero.com/2021/02/24/philosophy-of-software-design-summary/)
- [Single Level of Abstraction Principle](https://principles-wiki.net/principles:single_level_of_abstraction)
