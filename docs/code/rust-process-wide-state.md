---
name: "rust-process-wide-state"
description: "Process-wide `static` state in platform crates, and the `extern-state` feature that keeps it single when a program links more than one static library. Load when adding a process-wide `static` to a crate under `sys/`, when an application library outside the umbrella crate needs to reach one, or when a driver reports itself uninitialized after the program initialized it"
type: "arch"
scope: "global"
---

# Process-Wide State Across Static Libraries

**MANDATORY for ALL process-wide `static` state in crates reachable from more than one static library**

A platform crate holds state the C surface cannot pass: the socket driver's session, the descriptor
table, the mount table. A C caller writes `send(fd, ...)` and names no session, so the session has to
be ambient, and ambient means a `static`.

That works exactly as long as the crate is linked **once**.

## 1. Why Once Stops Being True

Every Rust crate here normally reaches a program through one umbrella static library, so there is one
compilation and one slot. An **application library** breaks that: it is compiled into an archive of
its own and linked alongside, and it takes the same platform crates as dependencies.

Cargo resolves features per build, and the two builds are separate `cargo build` invocations. They
disagree on at least one feature — the umbrella turns on `ffi`, an application library does not — so
they produce **two crate hashes**. A `static`'s symbol is mangled with that hash, so the program gets
**two slots**.

```
libnx_std.a         nx-sys-net (features: ffi)          → _RNvNtCsaJcf..._session_SERVICE
libnx_netloader.a   nx-sys-net (features: —)            → _RNvNtCsdVS7..._session_SERVICE
```

`socketInitialize` fills the first. `Socket::open` in the application library reads the second.

**The failure is silent.** Nothing is wrong at link time: the two symbols really are different
symbols. It surfaces at run time as a driver reporting itself uninitialized in a program that
demonstrably initialized it — which reads as a bug everywhere except where it is.

Matching the feature sets does not fix it. The dependency graphs still differ, so the hashes still
differ; and enabling `ffi` on both makes every `#[no_mangle]` entry point a duplicate-symbol error.

## 2. Why the Allocator and the Panic Handler Do Not Need This

They already have it. `#[global_allocator]` and `#[panic_handler]` are language items, and the
compiler emits them under **fixed, unmangled** symbols — `__rust_alloc`, `__rust_dealloc`,
`rust_begin_unwind`. One definition per program, enforced by the linker. `extern-state` is the same
trick applied by hand to state the language does not special-case.

What decides whether a `static` needs it is **how the state is reached**:

| Reached through | Single across archives? |
|---|---|
| A fixed unmangled symbol — every allocation goes through the one `__rust_alloc` | **Yes**, automatically |
| A generic or inlinable function, monomorphised into the *calling* crate | **No** — the copy in the caller reads the caller's own `static` |

`nx-sys-net`'s session is the second kind: `with_service::<T>` is generic, so it is compiled into
whoever calls it and references that crate's `SERVICE` directly, never passing through a shared
symbol. `nx-alloc`'s `ALLOC` is the first kind — nothing reaches it except the allocator shim behind
`__rust_alloc`, so one copy is pulled and one state is used.

This is also why the problem hides. A crate whose feature sets happen to match across both builds
gets one rlib from Cargo and one `static` for free, and looks fine right up until a feature diverges.
`nx-alloc` unifies today because both graphs ask for the same features; `nx-sys-net` does not,
because the umbrella adds `ffi`.

**Do not conclude from a working program that the state is shared.** Count the symbols:

```bash
nm <elf> | grep -c ' SERVICE$'      # 1 is shared; 2 is the silent failure waiting to happen
```

## 3. The Rule

**A process-wide `static` reachable from more than one static library takes a spelled-out symbol and
a feature that swaps its definition for a declaration.**

```rust
// ✅ Good — one definition in the archive that owns the resource, a declaration everywhere else.
#[cfg(not(feature = "extern-state"))]
#[unsafe(no_mangle)]
static SERVICE: RwLock<Option<BsdService>> = RwLock::new(None);

#[cfg(feature = "extern-state")]
unsafe extern "Rust" {
    static SERVICE: RwLock<Option<BsdService>>;
}
```

- **Exactly one** static library in a program leaves `extern-state` off. That one owns the resource
  and is the one whose initializer runs.
- **Every other** static library turns it on.
- Two definitions is a duplicate-symbol error; zero is an undefined-symbol error. Both are loud, and
  that is the point: the feature converts a runtime mystery into a link-time complaint.

## 4. Access Goes Through One Accessor

An `extern` static is `unsafe` to touch. Reaching it at every use site spreads `unsafe` across the
module for one invariant that is stated once.

```rust
// ✅ Good — the borrowed case costs one `unsafe`, where what is vouched for can be written down.
fn service() -> &'static RwLock<Option<BsdService>> {
    #[cfg(not(feature = "extern-state"))]
    {
        &SERVICE
    }

    #[cfg(feature = "extern-state")]
    // SAFETY: the symbol is defined by the one static library built without `extern-state`, as a
    // `RwLock<Option<BsdService>>` from this same source at this same version, so the reference has
    // the type and layout it claims. It is a `static`, so the `'static` lifetime is honest. The
    // lock orders access to what it holds; a shared reference to the lock itself races with nothing.
    unsafe {
        &*&raw const SERVICE
    }
}
```

## 5. `extern-state` Never Changes a Layout

The two builds agree on the type's layout because they are **the same source at the same version**.
That is the whole of the guarantee. A feature that added a field, reordered one, or changed a
generic parameter under `extern-state` would break it silently, in the worst possible way: two views
of one slot disagreeing about what is in it.

So `extern-state` gates a definition against a declaration and nothing else. It must not appear in
any `#[cfg]` that affects a type.

## 6. It Does Not Combine With `ffi`

A build with `extern-state` must not enable `ffi`. The C surface is `#[no_mangle]` throughout, so a
second archive carrying it collides on every entry point ([rust-ffi](rust-ffi.md)).

The division is the same one the state follows: **one archive owns the resource and exposes it to C;
the others borrow it from Rust.**

## 7. Prefer Not Needing It

This is a linker-level escape from a Cargo-level fact, and it is load-bearing in a way that is easy
to break from a distance. Before reaching for it, check whether the application library needs the
Rust API at all: the C ABI is single by construction, and a library that calls `socket()` and
`open()` through `extern "C"` has none of this problem.

Reach for `extern-state` when the Rust API is what the caller should have — typed addresses, owned
descriptors, `Result` — and the alternative is re-deriving it over raw descriptors.

## Checklist

Before committing code, verify:

- [ ] Every process-wide `static` reachable from a second static library is `#[unsafe(no_mangle)]`
      and gated by `extern-state`
- [ ] The crate declares an `extern-state` feature, documented as mutually exclusive with `ffi`
- [ ] Exactly one static library in each program leaves `extern-state` off
- [ ] All access goes through one accessor holding the single `unsafe`, with a `// SAFETY:` note
      naming the definition it resolves to
- [ ] `extern-state` appears in no `#[cfg]` that affects a type's layout
- [ ] The crate documentation explains which archive owns the state and why
- [ ] The C ABI was considered first, and the reason the Rust API is needed is written down
- [ ] The symbol count was checked in the linked binary (`nm <elf> | grep -c ' NAME$'` is 1), rather than
      inferred from the program appearing to work

## References

- [rust-ffi](rust-ffi.md) - Related: why `ffi` and `extern-state` cannot both be on
- [rust-crates](rust-crates.md) - Related: feature declaration and naming
- [principle-information-hiding](principle-information-hiding.md) - Related: the accessor is what
  keeps the storage private while the symbol is public
