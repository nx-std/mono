---
name: "principle-inversion-of-control"
description: "Inversion of Control — accept dependencies as parameters instead of constructing them. Load when designing components, wiring collaborators, or making code testable without external systems"
type: "principle"
scope: "global"
---

# Inversion of Control (Dependency Injection)

## Rule

A unit declares what it needs; it does not go and find it. Pass collaborators in — as function parameters, as
struct fields set by the constructor, or as generic parameters — instead of constructing them inside.

Inject when either signal fires:

1. **It performs I/O**: it issues a supervisor call, sends a request over an IPC session, allocates, draws from
   the random source, or reads the tick counter. Tests must be able to substitute it.
2. **It varies by context**: production and tests (or one deployment mode and another) need different
   instances.

If neither fires — an `ArrayVec`, a bitflags set, a pure helper in the same module — construct it inline.
Injecting it is noise.

Rust gives two shapes for the seam, and they are not interchangeable. A generic parameter (`T: Transport`)
monomorphizes, keeps the call devirtualized, and is the default for a collaborator fixed at construction. A
trait object (`&dyn Transport`) is for sets assembled at runtime or stored heterogeneously — a table of
registered providers, not a struct's single collaborator. In a `no_std` crate the owned form costs more than a
vtable hop: `Box<dyn Transport>` pulls the global allocator into a graph that may not have one, and puts an
indirect call on the marshalling hot path. Reach for `dyn` only when the set really is open at runtime.

## Examples

1. **Inject the effect, not the machinery that performs it**
   A client needs the server's reply to decode a record. It takes a transport.

```rust
// ❌ Bad — the supervisor call is baked in. To test "a reply shorter than the header is rejected
// rather than decoded", the test must run on a console with that service alive; on the host it
// will not even link. The decoding, which is where the edge cases live, ends up uncovered.
pub fn read_firmware_version(session: SessionHandle) -> Result<FirmwareVersion, ReadVersionError> {
    let mut buf = IpcBuffer::current();
    RequestHeader::for_command(3).write_into(&mut buf);
    nx_svc::ipc::send_sync_request(session)?;
    FirmwareVersion::decode(buf.as_bytes())
}
```

```rust
// ✅ Good — the one effect sits behind a trait bound, so the decoder is host-testable. A generic
// keeps the production path identical to the hand-written one: monomorphized, no vtable, and no
// allocator pulled into a crate that has none.
pub trait Transport {
    /// Sends the request staged in `buf` and leaves the raw reply in it.
    fn send_sync_request(&self, buf: &mut [u8]) -> Result<(), TransportError>;
}

pub fn read_firmware_version<T: Transport>(
    transport: &T,
    buf: &mut [u8],
) -> Result<FirmwareVersion, ReadVersionError> {
    RequestHeader::for_command(3).write_into(buf);
    transport.send_sync_request(buf)?;
    FirmwareVersion::decode(buf)
}

// The test — no framework, no kernel, just a struct that replays bytes:
struct CannedReply(&'static [u8]);

impl Transport for CannedReply {
    fn send_sync_request(&self, buf: &mut [u8]) -> Result<(), TransportError> {
        buf[..self.0.len()].copy_from_slice(self.0);
        Ok(())
    }
}

let mut buf = [0u8; 64];
assert!(read_firmware_version(&CannedReply(TRUNCATED_REPLY), &mut buf).is_err());
```

2. **Take the ambient value; do not reach for it**
   The tick counter, the random source, and the globals written at init are inputs. Read them at the entrypoint
   and pass them down.

```rust
// ❌ Bad — the tick source and the budget are read deep in the call graph. Testing "the poll gives
// up once the deadline passes" now requires a console and a real half-second wait, so the case is
// either skipped or written against a one-tick deadline and flakes whenever the core is preempted.
pub fn wait_until_ready(session: &Session) -> Result<(), WaitError> {
    let deadline = nx_svc::time::system_tick() + TICKS_PER_MS * 500;
    let budget = retry_budget(); // reads a static written during init
    // ...
}
```

```rust
// ✅ Good — the policy and the tick source arrive as arguments; the entrypoint reads the globals
// once. Tests advance a counter by hand and assert the give-up behaviour without waiting.
pub struct RetryPolicy {
    pub budget: u32,
    pub deadline: Tick,
}

pub fn wait_until_ready<T: Transport>(
    transport: &T,
    policy: RetryPolicy,
    now: impl Fn() -> Tick,
) -> Result<(), WaitError> {
    // the loop compares `now()` against `policy.deadline`; nothing here calls the kernel
}
```

## Why It Matters

Without injection, a unit's dependencies are invisible: nothing in `read_firmware_version(session)` says it
enters the kernel. With injection, the signature _is_ the dependency list, and a reviewer sees a function's
whole blast radius without reading its body.

It is also the only seam Rust gives you. There is no runtime patching here: an `nx_svc::ipc::send_sync_request`
buried in a function body cannot be replaced by a test, at any price. Either the collaborator is in the
signature or the code is reachable only through a hardware test — and hardware tests mean building an NRO,
deploying it to the console, and reading results off the screen, so they run at the end of a change rather than
inside the edit loop, and the awkward cases end up uncovered.

Injection is what keeps the dependency graph acyclic, too: the marshalling layer never depends on the service
wrappers layered on top of it, because sessions and transports arrive as values the caller composes and passes
in.

## Pragmatism Caveat

Do not inject for the sake of it. A service wrapper may open its own session from the service name it was given:
the session's lifetime is the wrapper's lifetime, no other implementation exists, and the hardware test drives
the real service anyway — the seam is the name, not the session. Injecting the value one level up (a service
name, a raw handle, a tick, a page count) is very often better than injecting the object.

Do not reach for `Box<dyn Trait>` where a generic parameter fits, and do not introduce a trait with exactly one
implementation and no test double: that is indirection bought with nothing, and in `no_std` it is indirection
bought with an allocator dependency.

When an I/O dependency is deliberately constructed inside, the seam that replaces it must exist and be named in
a comment. An I/O dependency with no seam at all is always wrong.

## Checklist

Before committing code, verify:

- [ ] Every I/O collaborator — kernel, session, allocator, clock, random source — is a parameter, a constructor
      argument, or a generic parameter, not constructed inline
- [ ] Effects a test needs to control are parameters (a transport, a tick source, a spec), with the real
      implementation supplied by the caller
- [ ] The tick counter, the random source, and statics written at init are read at entrypoints and passed down,
      not read deep in the call graph
- [ ] The unit can be exercised with a hand-written fake on the host, without a console and without an SVC
- [ ] Generic parameters are used for fixed collaborators; trait objects only where the set is runtime-assembled
- [ ] Where a dependency is constructed internally, the injectable seam (service name, handle, spec) is documented
- [ ] No trait exists solely to wrap a single implementation that nothing substitutes

## References

- [principle-law-of-demeter](principle-law-of-demeter.md) - Related: Injecting the exact value needed removes
  the reason to navigate an object graph
- [principle-single-responsibility](principle-single-responsibility.md) - Related: A unit with one responsibility
  has few enough collaborators to inject them all
- [principle-open-closed](principle-open-closed.md) - Related: The injection seam and the extension point are
  the same trait

## External References

- [Inversion of Control (Kent C. Dodds)](https://kentcdodds.com/blog/inversion-of-control)
- [Inversion of Control Containers and the Dependency Injection pattern (Martin Fowler)](https://martinfowler.com/articles/injection.html)
- [Beginner's Guide to Inversion of Control (HackerNoon)](https://hackernoon.com/beginners-guide-to-inversion-of-control)
