---
name: "pattern-builder"
description: "Builder pattern for complex object construction with required fields. Load when designing constructors with multiple required parameters or optional configuration"
type: "core"
scope: "global"
---

# Builder Pattern for Required Fields

## Rule

Use the builder pattern when construction has multiple required fields. The built type carries no `Option` fields for data that must always be present: the builder holds the optionality, and `build()` enforces completeness. A struct that exposes required data as `Option` because "it is set during construction" leaks its construction concerns into every consumer, and consumers must never unwrap fields that are guaranteed to exist.

## Examples

```rust
// ❌ Bad — easy to forget the entry point or the stack, and every consumer re-handles a None
// that the kernel will reject anyway: svcCreateThread aborts the console on a null stack.
pub struct ThreadAttr {
    pub entry: Option<ThreadEntry>,
    pub stack: Option<StackRegion>,
    pub priority: Priority,
    pub core_mask: CoreMask,
}

// Every caller must unwrap fields that a constructed ThreadAttr should always have
fn spawn(attr: &ThreadAttr) -> Result<ThreadHandle, CreateError> {
    let stack = attr.stack.as_ref().expect("missing stack"); // aborts under panic = "abort"
    svc_create_thread(attr.entry.unwrap(), stack, attr.priority, attr.core_mask)
}
```

```rust
// ✅ Good — the builder holds the optionality, the built type does not
pub struct ThreadAttr {
    entry: ThreadEntry,   // No Option — guaranteed to exist
    stack: StackRegion,   // No Option — guaranteed to exist
    priority: Priority,
    core_mask: CoreMask,
}

pub struct ThreadAttrBuilder {
    entry: Option<ThreadEntry>,
    stack: Option<StackRegion>,
    priority: Priority,
    core_mask: CoreMask,
}

impl ThreadAttrBuilder {
    pub fn new() -> Self {
        Self {
            entry: None,
            stack: None,
            priority: Priority::INHERIT,      // Optional: sensible default
            core_mask: CoreMask::DEFAULT,     // Optional: sensible default
        }
    }

    pub fn entry(mut self, entry: ThreadEntry) -> Self {
        self.entry = Some(entry);
        self
    }

    pub fn stack(mut self, stack: StackRegion) -> Self {
        self.stack = Some(stack);
        self
    }

    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn core_mask(mut self, core_mask: CoreMask) -> Self {
        self.core_mask = core_mask;
        self
    }

    // Yields a ThreadAttr whose stack is StackRegion, not Option<StackRegion> — nothing to unwrap
    // downstream, and the failure surfaces as an error here instead of an abort inside the SVC.
    pub fn build(self) -> Result<ThreadAttr, BuildError> {
        Ok(ThreadAttr {
            entry: self.entry.ok_or(BuildError::MissingEntry)?,
            stack: self.stack.ok_or(BuildError::MissingStack)?,
            priority: self.priority,
            core_mask: self.core_mask,
        })
    }
}
```

```rust
// ✅ Good — required fields enforced at compile time by distinct concrete stage types, so a
// forgotten stack fails the build instead of returning an error nobody reads on the console.
// Each stage is its own named struct rather than a `ThreadAttrBuilder<E, S>` carrying PhantomData
// markers: the stage name shows up in signatures, rustdoc, and compiler diagnostics, and each
// stage stores exactly the fields it already owns instead of threading type parameters everywhere.
pub struct ThreadAttrBuilder { priority: Priority, core_mask: CoreMask }

pub struct ThreadAttrWithEntry { entry: ThreadEntry, priority: Priority, core_mask: CoreMask }

pub struct ThreadAttrWithStack {
    entry: ThreadEntry,
    stack: StackRegion,
    priority: Priority,
    core_mask: CoreMask,
}

impl ThreadAttrBuilder {
    pub fn new() -> Self {
        Self { priority: Priority::INHERIT, core_mask: CoreMask::DEFAULT }
    }

    pub fn entry(self, entry: ThreadEntry) -> ThreadAttrWithEntry {
        ThreadAttrWithEntry { entry, priority: self.priority, core_mask: self.core_mask }
    }
}

impl ThreadAttrWithEntry {
    pub fn stack(self, stack: StackRegion) -> ThreadAttrWithStack {
        ThreadAttrWithStack {
            entry: self.entry,
            stack,
            priority: self.priority,
            core_mask: self.core_mask,
        }
    }
}

// build() exists only on the stage that has every required field — compile-time enforcement,
// and no Result to unwrap because there is nothing left to be missing.
impl ThreadAttrWithStack {
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn build(self) -> ThreadAttr {
        ThreadAttr {
            entry: self.entry,
            stack: self.stack,
            priority: self.priority,
            core_mask: self.core_mask,
        }
    }
}
```

## Why It Matters

Required data represented as `Option` forces every consumer to handle a `None` that should never occur. The builder isolates construction complexity in one place and produces a type that unconditionally guarantees its required fields, removing an entire class of runtime panics from unwrapping "always-present" fields.

## Pragmatism Caveat

Not every struct needs a builder. A struct with 2-3 required fields all available at construction time is clearer with a plain `new()`. Use a builder when construction is genuinely complex: many fields, a mix of required and optional, or an order that matters. Prefer a staged builder of distinct concrete types when misuse would be a serious bug; a runtime `build() -> Result` is fine for attribute-style objects where a clear error value suffices.

## Checklist

Before committing code, verify:

- [ ] Built types use concrete fields (not `Option`) for data that must always be present
- [ ] Builder's `build()` method validates all required fields are set
- [ ] Consumers of the built type never unwrap fields that the builder guarantees
- [ ] Simple structs with few required fields use `new()` instead of a builder
- [ ] Staged builders of distinct concrete types (not a generic builder carrying `PhantomData` markers) considered for
      safety-critical construction where compile-time enforcement is warranted

## References

- [principle-type-driven-design](principle-type-driven-design.md) - Foundation: Design principle this pattern implements
- [pattern-typestate](pattern-typestate.md) - Related: Type-state pattern used for compile-time builder enforcement
