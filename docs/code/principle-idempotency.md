---
name: "principle-idempotency"
description: "Idempotency — make operations safe to retry and replay. Load when writing extraction jobs, database writes, API handlers, resource startup/teardown, or retry logic"
type: "principle"
scope: "global"
---

# Idempotency (Safe Retries and Replays)

## Rule

Design state-altering operations so that running them twice has the same observable effect as running them
once. Whether explicit machinery is needed depends on who calls the operation:

1. **C-FFI entry points called by homebrew** (`__nx_*_initialize`, `__nx_*_exit`) → required. Several
   libraries in the same process each initialize the services they need; the same symbol _will_ be entered
   more than once, in an order you do not control.
2. **SVC wrappers that can return a transient result** (an interrupted wait, a cancelled request) → required.
   The caller re-issues the same call, and the second attempt must not consume state the first already took.
3. **Resource acquisition and release** (session open/close, `map`/`unmap`, `close_handle`, `Drop`) →
   required. Two callers must not open two sessions; a second `close` must not touch a recycled handle.
4. **Pure in-process functions** → nothing to do. `align_up(addr, page_size)` is idempotent by construction:
   same input, same output, no effects.

If re-running an operation opens a second session, closes a handle it no longer owns, unmaps a range twice, or
returns a different result, it is not idempotent. Count owners instead of storing a `bool`, take the state
before releasing it, and guard "already done" state.

## Examples

1. **Refcounted init/exit pair**
   A service client exports `initialize`/`exit` to C. Two libraries linked into the same NRO each call
   `initialize`, and whichever one finishes first calls `exit`.

```rust
// ❌ Bad — the second initialize opens a second session and overwrites the stored handle, leaking
// the first for the life of the process. Worse, the first exit closes the session while the other
// library is still dispatching on it: its next request landed on a slot the kernel had already
// recycled and came back 0xF601, three frames into rendering.
static SESSION: AtomicU32 = AtomicU32::new(raw::INVALID_HANDLE);

pub unsafe extern "C" fn __nx_wattctl_initialize() -> u32 {
    let session = match sm::open_session(SERVICE_NAME) {
        Ok(session) => session,
        Err(err) => return err.to_rc().to_raw(),
    };
    SESSION.store(session.to_raw(), Ordering::Release);
    0
}

pub unsafe extern "C" fn __nx_wattctl_exit() {
    let raw = SESSION.swap(raw::INVALID_HANDLE, Ordering::AcqRel);
    // SAFETY: `raw` was produced by the open_session above.
    unsafe { svc::close_handle(raw) };
}
```

```rust
// ✅ Good — the count is the synchronization point: N initializes open exactly one session, and only
// the Nth exit closes it. A failed open puts the count back, so the next call retries instead of
// handing out an invalid handle, and an unmatched exit is a no-op rather than an underflow.
static OWNERS: AtomicU32 = AtomicU32::new(0);
static SESSION: AtomicU32 = AtomicU32::new(raw::INVALID_HANDLE);

pub unsafe extern "C" fn __nx_wattctl_initialize() -> u32 {
    if OWNERS.fetch_add(1, Ordering::Acquire) > 0 {
        return 0; // Already initialized by an earlier caller.
    }
    match sm::open_session(SERVICE_NAME) {
        Ok(session) => {
            SESSION.store(session.to_raw(), Ordering::Release);
            0
        }
        Err(err) => {
            OWNERS.fetch_sub(1, Ordering::Release);
            err.to_rc().to_raw()
        }
    }
}

pub unsafe extern "C" fn __nx_wattctl_exit() {
    let previous = OWNERS.fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_sub(1));
    if previous != Ok(1) {
        return; // Other owners remain, or there were never any.
    }
    let raw = SESSION.swap(raw::INVALID_HANDLE, Ordering::AcqRel);
    // SAFETY: the count reached zero, so no caller can still be dispatching on this session, and
    // the handle was stored by the initialize that took the count from 0 to 1.
    unsafe { svc::close_handle(raw) };
}
```

The state must be a count, not a flag: a `bool` cannot tell "nobody initialized me" from "two libraries did",
and the difference is exactly the one that decides whether the handle may be closed.

2. **Take the state before releasing it**
   A mapped region is torn down from the failure path of setup, from an explicit release, and from `Drop`.

```rust
// ❌ Bad — the failure path unmaps and then the guard's Drop unmaps the same range again. The second
// unmap hit an address the reservation bitmap had already handed to another mapping, and the buffer
// living there went unreadable partway through the next present.
fn install_ring(guard: ShmemGuard) -> Result<Ring, InstallError> {
    match Ring::attach(guard.addr, guard.size) {
        Ok(ring) => Ok(ring),
        Err(err) => {
            // SAFETY: mapped by the caller that built the guard.
            unsafe { shmem::unmap(guard.handle, guard.addr, guard.size) };
            Err(err.into())
        }
    }
}

impl Drop for ShmemGuard {
    fn drop(&mut self) {
        // SAFETY: mapped when the guard was built.
        unsafe { shmem::unmap(self.handle, self.addr, self.size) };
    }
}
```

```rust
// ✅ Good — the guard owns the unmap and takes the region out of itself before performing it, so the
// failure path, an explicit release, and Drop all converge on exactly one unmap of one address.
struct ShmemGuard {
    handle: shmem::Handle,
    region: Option<Region>,
}

impl ShmemGuard {
    /// Unmaps the region. Safe to call again — a second call has nothing left to unmap.
    fn release(&mut self) {
        let Some(region) = self.region.take() else {
            return;
        };
        // SAFETY: `region` was mapped when the guard was built and has now been moved out of the
        // guard, so no other path — including Drop — can reach the same address again.
        unsafe { shmem::unmap(self.handle, region.addr, region.size) };
    }
}

impl Drop for ShmemGuard {
    fn drop(&mut self) {
        self.release();
    }
}
```

The same shape governs a retried SVC. A wait that comes back interrupted is re-issued, so the retry must not
restart anything the first attempt already consumed: recompute the remaining timeout from the monotonic tick
rather than passing the original one again, and treat "the event was already signalled" as success rather
than as a second acquisition. Wherever a resource guards "already done" state the rule is the same:
re-registering an already-registered event is a no-op, and an exit on a client that never initialized returns
cleanly instead of closing handle zero.

## Why It Matters

Every teardown path here runs more than once. A cleanup that fires from both an error branch and a `Drop` is
the normal case, not the exception, and an unguarded double release does not fail loudly — `close_handle` on a
recycled slot succeeds, against whatever object the kernel put there since. The corruption surfaces later, in
unrelated code, as a session that stopped answering for no reason anyone can trace back.

The same logic governs anything C can call twice. Homebrew links several libraries that each initialize the
services they need, so a client that assumes one caller opens a second session on the second call and leaks
it for the life of the process, because nothing holds a handle to it any more.

## Pragmatism Caveat

Some operations genuinely cannot be idempotent: popping an entry off a consuming queue twice is not the same
as popping it once. The response is not machinery but honesty — the operation says so in its docs, and the
wrapper that exposes it makes the consumption visible in its signature (taking `self`, or returning the
consumed item). An undocumented non-idempotent retry path is always wrong.

Equally, do not bolt refcounts onto pure functions or onto in-process calls where the caller controls
execution. A request encoder, a permission-flag conversion, and an alignment helper need nothing.

## Checklist

Before committing code, verify:

- [ ] Every `__nx_*_initialize`/`__nx_*_exit` pair reachable from C guards on an owner count (or equivalent
      "already done" state) instead of assuming a single caller
- [ ] Ownership state (counts, stored handles, `Option`s) is published in the same step as the resource it
      tracks, so no window exposes one without the other
- [ ] Lazily-opened resources are opened once behind a single guard, and a failed open leaves nothing recorded
- [ ] `exit()`/`close()`/`release()` take their state before touching the kernel, and are safe to call twice
- [ ] Cleanup on a failure path releases every resource it acquired, so one failing release cannot strand the
      rest
- [ ] Deliberately non-idempotent operations say so in their docs

## References

- [principle-validate-at-edge](principle-validate-at-edge.md) - Related: The C-FFI boundary that accepts a call
  is where its "already initialized" state is established
- [principle-least-surprise](principle-least-surprise.md) - Related: `initialize`/`exit`, `open`/`close` carry
  an implied "safe to call again" contract
- [principle-type-driven-design](principle-type-driven-design.md) - Related: Model "already released" as state
  the type carries, not as a fact the caller must remember

## External References

- [Idempotency in Depth (Luca Palmieri)](https://lpalmieri.com/posts/idempotency/)
- [Exactly-Once Semantics Are Possible (Confluent)](https://www.confluent.io/blog/exactly-once-semantics-are-possible-heres-how-apache-kafka-does-it/)
