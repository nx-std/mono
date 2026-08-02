---
name: "rust-errors-handling"
description: "Propagating and recovering errors: unwrap/expect ban, pattern matching, justifying a discarded error. Load when handling a Result or Option"
type: "core"
scope: "global"
---

# Rust Error Handling Patterns

**MANDATORY for ALL Rust code in the nx-std workspace**

## 1. Never `.unwrap()` or `.expect()` in Production

**ABSOLUTELY CRITICAL - ZERO TOLERANCE POLICY**

**DO NOT** use `.unwrap()` or `.expect()` in a production code path unless you can prove the operation cannot
fail. The workspace builds with `panic = "abort"`, so a panic ends the process outright: nothing unwinds, no
`Drop` runs, kernel handles stay open and mapped pages stay mapped until the kernel tears the process down,
and the panic message carries less about the failure than the error it discarded. A panic that reaches an
`extern "C"` boundary is undefined behaviour on top of that, so no `__nx_*` entry point may host one.

```rust
// ❌ Bad — a truncated reply or one out-of-range field takes down the process, and the panic
// reports less than the DecodeError it threw away
pub fn read_session_info(reply: &[u8]) -> SessionInfo {
    let header = CmifResponseHeader::read_from_prefix(reply).unwrap();
    SessionInfo::from_words(header.payload_words()).unwrap()
}

// ❌ Bad — a message does not make the panic acceptable; this still aborts the process
pub fn read_session_info(reply: &[u8]) -> SessionInfo {
    let header = CmifResponseHeader::read_from_prefix(reply).expect("failed to read header");
    SessionInfo::from_words(header.payload_words()).expect("failed to decode session info")
}

// ✅ Good — the caller decides what a truncated or malformed reply means
pub fn read_session_info(reply: &[u8]) -> Result<SessionInfo, ReplyError> {
    let header = CmifResponseHeader::read_from_prefix(reply).map_err(ReplyError::HeaderTruncated)?;
    let info = SessionInfo::from_words(header.payload_words()).map_err(ReplyError::PayloadInvalid)?;
    Ok(info)
}
```

**Code review red flag:** any `.unwrap()` or `.expect()` in a production path is rejected unless a code
invariant makes the failure impossible, and that invariant is written down where it is assumed:

1. **Proof of safety** - a logical analysis or type-system guarantee that the operation cannot fail
2. **`// SAFETY:` comment on the full statement** - placed immediately above the statement that holds the
   `.unwrap()`/`.expect()` (not on the message string), naming the invariant that makes the panic impossible,
   so a reviewer can check the claim and a later change to the invariant has a searchable site to re-examine

Do **not** add a `# Panics` rustdoc section for such a call. The invariant makes it unreachable, so a `# Panics`
section would document a failure that cannot occur and mislead the caller. `# Panics` is for a panic that a
caller can actually trigger ([rust-docs-rustdoc](rust-docs-rustdoc.md)); a provably-unreachable `.expect()` is
not one. If the failure *can* happen, the call is not provably safe: return a `Result` instead.

**Even when it is provably safe, prefer refactoring to eliminate it entirely.**

```rust
// ✅ Good — the invariant sits above the statement, not buried in the expect message, and no
// # Panics section is written because the window is const-sized and the buffer length is fixed.
// SAFETY: `self.words` is a `[u32; 8]`, so `raw` is always 32 bytes and the 4-byte window at
// HANDLE_OFFSET (8) is always in bounds; the array conversion therefore cannot fail.
let handle = u32::from_le_bytes(<[u8; 4]>::try_from(&raw[8..12]).expect("4-byte window in bounds"));
```

A genuinely fallible construction is the case that fits neither. Opening a session to a Horizon service can
fail when `sm` has no such service registered yet or the process has run out of session handles, and no code
invariant rules that out. A `// SAFETY:` comment would assert something the code does not guarantee, and a
`# Panics` section would document a panic the caller can neither provoke through its arguments nor prevent.
The honest form is a `Result` the caller propagates, built where a constructor already returns one.

```rust
// ❌ Bad — the failure is real (the service may be unregistered, or the process out of sessions),
// so the SAFETY comment overclaims and the `.expect()` turns a runtime condition into an abort.
// SAFETY: the session always opens.
let session = sm::open_session(ServiceName::new("fsp-srv")).expect("fsp-srv session opens");

// ✅ Good — the constructor is fallible, so the caller decides what an unavailable service means.
fn with_default_session() -> Result<Self, sm::OpenSessionError> {
    let session = sm::open_session(ServiceName::new("fsp-srv"))?;
    Ok(Self::new(session))
}
```

## 2. Prefer Pattern Matching

**ALWAYS** handle `Result` and `Option` by matching. The type system is your ally - use it.

### `let-else` for an Early Return

```rust
// ✅ Good — the failure exits immediately and `slot` is a value, not an Option, below
pub fn get_event(&self, id: EventId) -> Result<EventHandle, GetEventError> {
    let Some(slot) = self.slots.get(id.index()) else {
        return Err(GetEventError::UnknownEvent { id });
    };

    slot.handle().ok_or(GetEventError::EventClosed { id })
}
```

### `match` for Multiple Cases

```rust
// ✅ Good — each failure maps to the result code it deserves, and a new DispatchError
// variant breaks the build instead of silently taking the catch-all
pub fn to_result_code(result: Result<Reply, DispatchError>) -> ResultCode {
    match result {
        Ok(_) => rc::SUCCESS,
        Err(DispatchError::SessionClosed) => KernelError::SessionClosed.to_rc(),
        Err(DispatchError::ReplyTooLarge) => KernelError::MessageTooLarge.to_rc(),
        Err(err) => err.to_rc(),
    }
}
```

### `if let` for a Single Case

```rust
// ✅ Good — the one failing case is handled where it happens and the caller is not burdened
pub fn release_all(&mut self, handles: &[Handle]) {
    for handle in handles {
        if let Err(err) = close_handle(*handle) {
            self.leaked.push((*handle, err));
        }
    }
}
```

### Combinators for Transformation Chains

```rust
// ✅ Good — the absent case produces a value instead of a branch
pub fn service_name(&self, id: ServiceId) -> &str {
    self.services
        .get(id.index())
        .map(|service| service.name())
        .unwrap_or("<unregistered>")
}

// ✅ Good — `ok_or` turns a missing value into the error that names it
pub fn require_session(session: Option<Session>) -> Result<Session, Error> {
    session.ok_or(Error::SessionMissing)
}
```

## 3. A Discarded Error Carries a Justification

Dropping a `Result` — `let _ = fallible();`, `.ok()`, an `Err(_)` arm that does not propagate — is a decision,
and it is invisible unless it is written down. The next reader cannot tell a considered discard from a bug,
so every one carries a comment naming **what would break if the error escaped**.

```rust
// ❌ Bad — a silent discard. Nothing distinguishes this from a forgotten `?`,
// and the failure it swallows never appears anywhere.
let _ = svc::close_handle(self.handle);
```

```rust
// ✅ Good — the comment names what the discard protects, so the cost of the lost
// error can be weighed.
// `Drop` has no caller to return a result code to, and the only failure the kernel
// reports here is InvalidHandle — the handle is already gone, which is the outcome
// this drop wanted. Panicking instead would abort the process during teardown.
let _ = svc::close_handle(self.handle);
```

Discarding is not a way to avoid handling an error. If the failure matters to the caller, propagate it. The
comment is only for the case where losing it is genuinely correct. What the comment must say is governed by
[rust-docs-comments](rust-docs-comments.md).

## 4. Test Code Exception

**EXCEPTION**: `.expect()` with a descriptive message is **acceptable and recommended in test code**. A test
should fail loudly when a precondition is not met, and the message names which one.

Message format: `"<operation> should <expected behavior>"`. Never use `.unwrap()`, even in tests.

```rust
// ✅ Good — a failure names the step that broke without opening the test
#[test]
fn test_event_creation_with_valid_flags_succeeds() {
    //* Given
    let table = EventTable::new();
    let flags = EventFlags::AUTO_CLEAR;

    //* When
    let event_id = table
        .create_event(flags)
        .expect("event creation should succeed with valid flags");

    //* Then
    let retrieved = table
        .find_event(event_id)
        .expect("should query the event table")
        .expect("event should exist in the table");

    assert_eq!(retrieved.id, event_id);
}

// ❌ Bad — the panic points at a line number and nothing else, so a red run on the console
// does not say whether creation, lookup, or existence failed
#[test]
fn test_event_creation() {
    let table = EventTable::new();
    let event_id = table.create_event(flags).unwrap();
    let retrieved = table.find_event(event_id).unwrap().unwrap();
    assert_eq!(retrieved.id, event_id);
}
```

## Checklist

Before committing Rust code, verify:

- [ ] **ZERO `.unwrap()` calls in production code paths**
- [ ] **ZERO `.expect()` calls in production code (except provably safe with documentation)**
- [ ] Pattern matching used for all `Result` and `Option` handling
- [ ] `let-else` used for early returns from `Option`
- [ ] `match` used for explicit multi-branch handling
- [ ] `if let` used for single-case handling
- [ ] Combinators (`.map()`, `.ok_or()`, `.and_then()`) used appropriately
- [ ] Every discarded `Result` (`let _ =`, `.ok()`, a non-propagating `Err(_)` arm) carries a comment naming
      what would break if the error escaped
- [ ] Test code uses `.expect()` with descriptive messages (NOT `.unwrap()`)
- [ ] Every unwrap/expect in production code carries a `// SAFETY:` comment on its full statement naming the
      invariant that makes the panic impossible, plus a logical or type-system proof of safety
- [ ] No `# Panics` section is added for a provably-unreachable unwrap/expect; the `// SAFETY:` comment is what
      documents it
- [ ] Functions return `Result<T, E>` for all fallible operations
- [ ] Error types provide rich context (see [rust-errors-reporting](rust-errors-reporting.md))
- [ ] No panic-inducing code without documentation and proof

## References

- [rust-errors-reporting](rust-errors-reporting.md) - Related: Declaring the error types propagated here
- [rust-docs-rustdoc](rust-docs-rustdoc.md) - Related: Owns the `# Panics` section, and when a provably-safe
  unwrap/expect omits it
