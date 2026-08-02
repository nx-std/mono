---
name: "principle-type-driven-design"
description: "Type-Driven Design — make illegal states unrepresentable with enums and newtypes. Load when designing data types or reviewing optional fields that allow invalid combinations"
type: "principle"
scope: "global"
---

# Type-Driven Design (Make Illegal States Unrepresentable)

## Rule

Design types so invalid states cannot be constructed. Parse at the boundary, then let every downstream function
receive a type that structurally rules out the cases it does not handle.

Concretely:

- A struct with several `Option` fields where only some combinations are legal is wrong. Replace it with an
  **enum** whose variants are exactly the legal shapes.
- Model "succeeded, or here is why not" as an **enum returned by value** or a `Result` with a typed error enum
  — not as `Option<T>` plus an out-of-band message, and not as a string the caller is expected to inspect.
- Let the compiler enforce it. Match exhaustively rather than with a catch-all arm on an enum you own: the
  catch-all is what silently absorbs the variant added next year. Index with `.get(i)` when the index is
  data-derived; `slice[i]` asserts a bound the type does not carry.
- A newtype's invariant is established in its validating constructor — `FromStr` from a string, `TryFrom`
  from any other type — not by the caller. Constructing one from a raw value with
  an unchecked constructor asserts the fact the newtype exists to prove, so those constructors carry a
  `// SAFETY:` comment naming why the invariant already holds.

If a function starts with defensive checks for a state that "shouldn't happen", the type is letting it happen.

## Examples

1. **Enum over co-optional fields**
   The service manager answers "here is a session, or here is the result code saying why you get none".
   Both-set and neither-set are meaningless.

```rust
// ❌ Bad — four representable states, two of them nonsense (both set; neither set).
// Every caller must defensively check both fields, and nothing forces it to. The
// version that shipped read `session` first and aborted on the unregistered path.
pub struct QueryOutcome {
    pub session: Option<SessionHandle>,
    pub rejected: Option<ResultCode>,
}

let outcome = manager.query(port);
if let Some(code) = outcome.rejected {
    return Err(ConnectError::Rejected(code));
}
let session = outcome.session.unwrap(); // the compiler cannot help here
```

```rust
// ✅ Good — exactly two states; the match hands the caller a non-optional session.
pub enum QueryOutcome {
    Connected(SessionHandle),
    Rejected { code: ResultCode },
}

let session = match manager.query(port) {
    QueryOutcome::Connected(session) => session,
    QueryOutcome::Rejected { code } => return Err(ConnectError::Rejected(code)),
};
```

2. **Parse once into a newtype; do not re-assert the invariant downstream**
   A port name is not a `&str`. Validate it where it enters and carry the proof in the type.

```rust
// ❌ Bad — a raw &str for validated protocol data. Is it ASCII? Is it at most the
// eight bytes the kernel copies into a register? Every function downstream either
// re-checks or trusts blindly, and this one panics on every name shorter than 8.
pub fn port_word(name: &str) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&name.as_bytes()[..8]);
    u64::from_le_bytes(word)
}
```

```rust
// ✅ Good — one validating constructor; downstream signatures state what they require.
pub struct PortName([u8; 8]);

impl core::str::FromStr for PortName {
    type Err = ParsePortNameError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let bytes = input.as_bytes();
        if bytes.len() > 8 {
            return Err(ParsePortNameError::TooLong { len: bytes.len() });
        }
        if !input.is_ascii() {
            return Err(ParsePortNameError::NotAscii);
        }
        let mut padded = [0u8; 8];
        padded[..bytes.len()].copy_from_slice(bytes);
        Ok(Self(padded))
    }
}

// No defensive check: the type already carries the proof.
pub fn port_word(name: &PortName) -> u64 {
    u64::from_le_bytes(name.0)
}
```

## Why It Matters

Every illegal state a type permits becomes a defensive check somewhere — or, more often, a missing defensive
check and, with `panic = "abort"`, a homebrew that dies on the console with a fatal code and no backtrace.
`session: Option<SessionHandle>` pushes an absence check onto every call site; an enum forces the caller to
handle the failure _once_, at the match, and hands them a non-optional value afterwards.

It also decides what your errors can say. A `&str` that might be a port name produces "invalid argument" from
somewhere deep inside a marshalling routine; a `PortName` that failed to parse produces a typed error at the
edge, naming the offending string and the connection attempt that carried it, before any SVC was issued.

## Pragmatism Caveat

Encode structural invariants, not policy. "A request buffer is a send buffer or a receive buffer, never both"
is structural — put it in the type. "A dispatch retries at most twice after a session reset" is policy that
will change — keep it a runtime check.

The same test decides when a primitive earns a newtype. A value with an invariant, a unit, or a same-typed
sibling it must never be swapped with is structural: a thread handle and an event handle, a byte offset and a
word index into the request payload, a virtual address and the page-aligned base derived from it. Those get
newtypes, validated in `TryFrom` (or `FromStr` where the value arrives as text) and constructed at the boundary
the value enters through — the `pattern-newtype` rule document governs them. A value with no invariant and
nothing to confuse it with — a debug label, a free-form panic message — stays a plain `&str`; a newtype there
is ceremony.

Casting **into** a validated type is the same error wearing a nominal type: `PageAddr(raw_addr)` from an
unaligned source asserts exactly what the newtype exists to prove. Where an unchecked constructor is
genuinely warranted, it carries a `// SAFETY:` comment naming the reason the invariant already holds.

## Checklist

Before committing code, verify:

- [ ] No struct has two or more `Option` fields whose combinations include meaningless states
- [ ] "Succeeded or here's why not" is an enum or a `Result` with a typed error, not `Option` plus a message
- [ ] Matches on enums the workspace owns are exhaustive, not closed with a catch-all arm
- [ ] Data-derived indexing uses `.get()` and discharges the absence; `slice[i]` is used only where the bound
      is structurally guaranteed
- [ ] A primitive with an invariant, a unit, or a same-typed sibling it must not be swapped with is a newtype;
      one with neither stays plain
- [ ] Newtype invariants are established in `FromStr` or `TryFrom`; every unchecked constructor carries a
      `// SAFETY:` comment
- [ ] No defensive runtime check re-verifies something the type already guarantees

## References

- [principle-validate-at-edge](principle-validate-at-edge.md) - Related: Boundary parsing is what produces the
  validated types this principle relies on
- [principle-least-surprise](principle-least-surprise.md) - Related: A well-named type whose shape lies is worse
  than no type
- [principle-law-of-demeter](principle-law-of-demeter.md) - Related: Returning an enum lets a collaborator
  answer completely instead of exposing internals

## External References

- [Designing with Types: Making Illegal States Unrepresentable (F# for Fun and Profit)](https://fsharpforfunandprofit.com/posts/designing-with-types-making-illegal-states-unrepresentable/)
- [Parse, Don't Validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [Parse, Don't Validate and Type-Driven Design in Rust](https://www.harudagondi.space/blog/parse-dont-validate-and-type-driven-design-in-rust/#maxims-of-type-driven-design)
- [The Ultimate Guide to Rust Newtypes](https://www.howtocodeit.com/guides/ultimate-guide-rust-newtypes)
- [Using Types To Guarantee Domain Invariants](https://lpalmieri.com/posts/2020-12-11-zero-to-production-6-domain-modelling/)
