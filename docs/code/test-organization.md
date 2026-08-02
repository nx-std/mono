---
name: "test-organization"
description: "Unit, on-hardware and e2e tiers with the it_* convention and host vs console selection. Load when deciding test type or placement"
type: "core"
scope: "global"
---

# Test Organization

**MANDATORY for choosing the tier and variant of any new test**

## Three Tiers

| Tier | Dependencies | Speed | Purpose | Where it runs |
|------|-------------|-------|---------|---------------|
| **Unit** | None | Milliseconds | Pure logic: bit packing, arithmetic, parsing | Host, `cargo test` |
| **On-hardware** | Live Horizon kernel | Seconds | Code that issues SVCs or holds real handles | Switch/emulator, in the NRO |
| **E2E** | Live Horizon kernel | Seconds | Cross-crate workflows through the FFI surface | Switch/emulator, in the NRO |

Each tier has a role the others cannot fill: unit tests cannot verify that an SVC returns the handle the kernel
actually minted, on-hardware tests cannot verify that several crates compose across the `__nx_*` boundary, and e2e
tests are too slow and too broad to pin down a single off-by-one in a header field. Start with unit tests for pure
logic, use on-hardware tests for anything that needs a kernel, and use e2e tests for cross-crate workflows.

For how to **run** tests (justfile tasks, building and deploying the NRO, per-crate commands), see the `/code-test`
and `/code-deploy` skills.

## In-tree vs Public API Variants

The unit and on-hardware tiers each split into two variants based on **where** the test lives and **what** it can
access:

| Variant | Location | API Access | Distinguishing Convention |
|---------|----------|------------|--------------------------|
| **In-tree** | `src/` (`#[cfg(test)]` modules) | Internal + public APIs | Unit: `tests::` (no `it_*`), On-hardware: `tests::it_*` |
| **Public API** | `<crate>/tests/` directory | Public API only | Unit: no `it_*` prefix, On-hardware: `it_*` prefix |

The `it_*` prefix is the **sole mechanism** that distinguishes on-hardware tests from unit tests in both locations.
Tests without `it_*` are host-runnable unit tests; tests with `it_*` need a live kernel. This lets a host run select
everything *except* `it_*` and the console run select `it_*`, with one naming rule instead of a hand-maintained list.

In-tree tests reach internal APIs, which is what makes it possible to test non-public packing helpers, internal
cursors, and error paths in internal components. Public API tests verify the external contract: that the exported
interface is ergonomic and correct, that documented sequences work, and that errors propagate through it. Location
determines API access; the `it_*` prefix determines whether a kernel is required.

## Unit Tests

Unit tests must have **no kernel dependency** and execute in **milliseconds**. They validate pure logic — wire-format
packing, bit fields, alignment arithmetic, result-code decomposition, buffer layout planning — without SVCs, handles,
or a live service session.

- **No kernel dependency**: no SVC, no handle, no service session, no allocator against the loader heap
- **Performance**: must complete in milliseconds
- **Reliability**: 100% deterministic, no flakiness
- **No `it_*` prefix**: test functions and modules must NOT use the `it_*` naming convention

Variants:

- **In-tree**: `src/` files inside `#[cfg(test)] mod tests { ... }`; internal and public APIs; selected by the host
  run, which excludes any `it_*` path
- **Public API**: `<crate>/tests/` files without an `it_*` prefix; public API only (separate crate); also selected by
  the host run

What to cover: wire-format encoding and decoding (IPC header packing, descriptor words, tag fields), bit-field
accessors (round-tripping every field, masking and shift boundaries), address arithmetic (page rounding, alignment
up/down, overflow at the top of the address space), result-code decomposition (module and description extraction,
success and failure discrimination), and buffer layout planning (how many descriptors a request needs and at what
offsets).

```rust
// ✅ Good — in-tree unit test: pure bit packing, no it_ prefix, so the host run picks it up
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_header_with_two_send_buffers_sets_descriptor_counts() {
        //* Given
        let layout = BufferLayout { send: 2, recv: 0 };

        //* When
        let word = pack_header_word(CommandType::Request, layout);

        //* Then
        assert_eq!(word >> 20 & 0xf, 2, "send descriptor count should occupy bits 20..24");
        assert_eq!(word >> 24 & 0xf, 0, "recv descriptor count should be zero");
    }
}

// ✅ Good — public API unit test in tests/page_math.rs: no it_ prefix, exported items only
use nx_sys_mem::PageAligned;

#[test]
fn align_up_at_page_boundary_is_idempotent() {
    //* Given
    let already_aligned = 0x8000_0000_usize;

    //* When
    let value = PageAligned::align_up(already_aligned);

    //* Then
    assert_eq!(value.get(), already_aligned, "an aligned address must not move");
}
```

## On-Hardware Tests

On-hardware tests verify that components work correctly against a **live Horizon kernel**: real SVCs, real handles,
real service sessions, the allocator against the loader heap, real threads. They cannot run on the host — they are
compiled into the homebrew test NRO under `subprojects/tests/` and executed on a console or emulator.

- **Kernel dependency**: real SVC calls, kernel-owned handles, live `sm` sessions, actual memory mappings
- **Mandatory `it_*` prefix on the parent module**: on-hardware tests must live inside an `it_*`-prefixed module or
  file so the host run can exclude them and the console run can select them
- **Flakiness risk**: may fail due to console state (a service already claimed, address-space pressure, a stale
  session from a previous run)
- **Performance**: seconds, plus a build-and-deploy cycle

Variants:

- **In-tree**: `src/` files inside `tests::it_*` submodules, either as separate files `src/<module>/tests/it_*.rs` or
  inline `mod it_*` submodules; internal and public APIs; selected by the console run
- **Public API**: `<crate>/tests/` files with an `it_*` prefix; public API only (separate crate); selected by the
  console run

What to cover: SVC behavior (return codes for out-of-range arguments, handle lifetimes, memory permission changes),
service sessions (acquiring a port from `sm`, issuing a command, decoding the response), allocator behavior against
the real loader heap (large allocations, alignment guarantees, growth and release), synchronization primitives under
real threads (contention, wake ordering, timeouts), and virtual-memory reservation against the live address space.

```rust
// ✅ Good — in-tree on-hardware test: needs a real handle, so it sits under an it_ module
#[cfg(test)]
mod tests {
    use super::*;

    mod it_transfer_memory {
        use super::*;

        #[test]
        fn create_transfer_memory_with_page_multiple_size_returns_handle() {
            //* Given
            let backing = ReservedPages::reserve(2).expect("should reserve two pages");

            //* When
            let result = TransferMemory::create(backing.as_range(), Permission::ReadWrite);

            //* Then
            let tmem = result.expect("kernel should mint a transfer memory handle");
            assert!(tmem.raw_handle() != 0, "a live handle must be non-zero");
            tmem.close().expect("closing a freshly created handle should succeed");
        }
    }
}

// ✅ Good — public API on-hardware test in tests/it_api_session.rs: it_ prefix, exported items only
use nx_sf::{Service, ServiceName};

#[test]
fn acquire_service_and_issue_command_succeeds() {
    //* Given
    let name = ServiceName::new("fsp-srv").expect("should build a valid service name");
    let service = Service::acquire(&name).expect("sm should hand back a session");

    //* When
    let result = service.dispatch_in::<u64>(1, ());

    //* Then
    let pid_placeholder = result.expect("command 1 should return a value");
    assert_ne!(pid_placeholder, 0, "the service should populate the response payload");
    service.close().expect("closing a live session should succeed");
}
```

```rust
// ❌ Bad — kernel-dependent test without the it_ prefix: the host run tries to execute it and the
// process dies on an unimplemented SVC, so the whole host suite reports a failure nobody can fix on the host
#[test]
fn create_transfer_memory_returns_handle() {
    let tmem = TransferMemory::create(range, Permission::ReadWrite).expect("should create");
    assert!(tmem.raw_handle() != 0);
}
```

```rust
// 🔶 Acceptable — a pure-logic test kept under an it_ module because it shares fixture setup with its
// hardware siblings; it costs a deploy cycle to run something the host could have checked in milliseconds
mod it_descriptor {
    #[test]
    fn descriptor_offsets_are_monotonic() {
        // pure arithmetic, no kernel — move it out once the fixture is no longer shared
    }
}
```

## E2E Tests

E2E tests live in the homebrew test package under `subprojects/tests/`, not in individual crate `tests/` directories.
They exercise cross-crate, end-to-end workflows through the built NRO: the C harness links against the Rust crates,
the linker scripts redirect `libnx` symbols to the `__nx_*` implementations, and the whole stack runs on the console.
They require a full environment (a booted console or emulator, a deploy step) and are selected by building and
running the test NRO rather than by a test-name filter.

What to cover: cross-crate workflows (an allocation flowing from `nx-alloc` through `nx-sys-mem` down to `nx-svc`),
FFI-surface integration (C callers reaching Rust implementations through the override scripts), and complete
scenarios (spawn a thread, hand it work through a `nx-std-sync` primitive, join it, and tear everything down).

## Checklist

When deciding which test tier and variant to use:

- [ ] Is the logic pure (packing, arithmetic, parsing) with zero kernel dependency? → **Unit test**
- [ ] Does the code need an SVC, a real handle, a live session, or the loader heap? → **On-hardware test** (use `it_*` prefix)
- [ ] Does the test span multiple crates end-to-end through the FFI surface? → **E2E test** (`subprojects/tests/` NRO)
- [ ] Does the test need access to internal APIs? → **In-tree** variant (in `src/`)
- [ ] Should the test only use the public API? → **Public API** variant (in `<crate>/tests/`)
- [ ] Is the test fast (milliseconds) and host-runnable? → Unit test
- [ ] Is the test slow (seconds) and only meaningful on a console? → On-hardware test with `it_*` prefix

## References

- [test-files](test-files.md) - Related: Where test modules and files live in the directory structure
- [test-functions](test-functions.md) - Related: Naming, Given-When-Then structure, and assertions inside a test function
