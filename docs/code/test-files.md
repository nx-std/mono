---
name: "test-files"
description: "Test file placement, cfg(test) modules, it_* naming, in-tree vs tests/ directory. Load when creating test files or organizing test modules"
type: "core"
scope: "global"
---

# Test Files

**MANDATORY for placing test modules and test files in any crate**

## Canonical Layout

```
<crate-root>/
  src/
    module.rs              # Source + #[cfg(test)] mod tests { ... }
    module/
      tests/
        validation.rs      # Unit tests (NO it_ prefix)
        it_session.rs      # In-tree integration tests (it_ prefix)
  tests/
    it_api_session.rs      # Public API integration tests (it_ prefix)
```

The `it_` prefix is the **sole mechanism** that distinguishes integration tests (need the real environment: a live
kernel, a service session, a mapped page) from unit tests (pure logic on the host, milliseconds).

## Unit Test Placement

Unit tests have **no external dependencies** and execute in **milliseconds**. They validate pure logic, wire-format
encoding and decoding, address arithmetic, and error-code mapping — everything that runs on the host toolchain
without a console.

### Co-located Tests

Tests live in the same file as the code, inside a `#[cfg(test)]` module. Use this when the module has few tests (under ~50 lines), the tests are simple, and no complex fixtures are needed.

```rust
// ✅ Good — tests sit next to the code they cover, so they get updated when it changes
fn parse_service_name(name: &str) -> Result<ServiceName, NameError> {
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.len() > MAX_NAME_LEN {
        return Err(NameError::TooLong);
    }
    Ok(ServiceName::from_ascii(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod validation {
        use super::*;

        #[test]
        fn parse_service_name_with_valid_input_succeeds() {
            //* Given
            let valid_name = "fsp-srv";

            //* When
            let result = parse_service_name(valid_name);

            //* Then
            assert!(result.is_ok(), "parsing should succeed with a valid name");
            assert_eq!(result.expect("should return service name").as_str(), valid_name);
        }
    }
}
```

`#[cfg(test)]` keeps test code out of production binaries, so co-location costs nothing at runtime.

### In-tree `tests/` Directory

Extract tests to `src/<module>/tests/` when the suite grows past ~50 lines, needs complex fixtures or setup, or spans several files for one module.

Unit test files and modules there **MUST NOT** start with `it_`.

```rust
// ✅ Good — src/session/tests/validation.rs carries no it_ prefix, so it stays in the unit profile
use crate::session::*;

mod unit_validation {
    use super::*;

    #[test]
    fn parse_service_name_with_empty_input_fails() {
        //* Given
        let empty_name = "";

        //* When
        let result = parse_service_name(empty_name);

        //* Then
        assert!(result.is_err(), "parsing should fail with an empty name");
        let error = result.expect_err("should return name error");
        assert!(matches!(error, NameError::Empty),
            "Expected Empty error, got {:?}", error);
    }
}

mod decoding_functions {
    use super::*;

    #[test]
    fn decode_cmif_header_with_valid_words_succeeds() { /* ... */ }
}
```

## In-tree Integration Test Placement

In-tree integration tests cover **internal functionality** not exposed through the crate's public API, and they need
the **real environment** — a live kernel, an open service session, a mapped memory region. They are built into the
on-hardware NRO suite and run on console or emulator, never on the host.

Their module or file name **MUST** start with `it_`.

### Inline Integration Submodule

Use when the tests are closely tied to the implementation and few in number.

```rust
// ✅ Good — integration tests sit beside the code they cover, isolated in an it_ submodule
pub fn query_pointer_buffer_size(
    session: &Session,
    request_id: u32,
) -> Result<u16, DispatchError> {
    /* ... */
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for pure functions here...

    mod it_pointer_buffer {  // ✅ Good — the it_ prefix keeps kernel-backed tests out of the unit profile
        use super::*;
        use crate::temp::temp_session;

        #[test]
        fn query_pointer_buffer_size_with_open_session_succeeds() {
            //* Given
            let session = temp_session("fsp-srv");
            let request_id = CONTROL_QUERY_POINTER_BUFFER_SIZE;
            let handshake = session.wait_ready();
            assert!(handshake.is_ok(), "session handshake should succeed");

            //* When
            let result = query_pointer_buffer_size(&session, request_id);

            //* Then
            assert!(result.is_ok(), "pointer buffer query should succeed");
            let size = result.expect("should return buffer size");
            assert!(
                size > 0,
                "an open session should report a non-zero pointer buffer"
            );
        }
    }
}
```

### External Integration Test File

Use for large integration suites, complex setup that warrants a dedicated file, or several integration files for one module.

```rust
// ✅ Good — src/session/tests/it_session.rs uses the it_ prefix required for filtering
use crate::session::*;
use crate::temp::temp_session;

#[test]
fn query_pointer_buffer_size_with_open_session_succeeds() {
    //* Given
    let session = temp_session("fsp-srv");
    let request_id = CONTROL_QUERY_POINTER_BUFFER_SIZE;

    //* When
    let result = query_pointer_buffer_size(&session, request_id);

    //* Then
    assert!(result.is_ok(), "pointer buffer query should succeed");
}
```

## Public API Integration Test Placement

Public API integration tests verify **end-to-end functionality** through the **crate's public API only**, using Rust's standard `<crate-root>/tests/` directory (outside `src/`). Each file compiles as a separate crate, so no internal API is reachable. These tests may need the real environment.

Files there **MUST** be named `it_*`.

```rust
// ✅ Good — tests/it_api_session.rs exercises only exported items, so refactoring internals cannot break it
use nx_sf::{ServiceName, Session, SessionManager};
use nx_sf::temp::temp_manager;

#[test]
fn open_session_and_dispatch_request_workflow_succeeds() {
    //* Given
    let manager = temp_manager();
    let name = ServiceName::new("fsp-srv")
        .expect("should create valid service name");
    let open_result = manager.open(&name);
    assert!(open_result.is_ok(), "service lookup should succeed");

    //* When
    let session = open_result.expect("should return an open session");
    let reply = session.dispatch(REQUEST_GET_TOTAL_SPACE, &[]);

    //* Then
    assert!(reply.is_ok(), "request dispatch should succeed");
    let reply = reply.expect("should return a reply");
    assert_eq!(reply.service_name(), name);
    assert_eq!(reply.raw_data_len(), size_of::<u64>());
}
```

## The it_ Naming Convention

| Test Type | Location | Naming Rule | Example |
|-----------|----------|-------------|---------|
| **Unit** (no external deps) | `#[cfg(test)] mod tests` | **NO** `it_` prefix | `mod validation` |
| **Unit** (no external deps) | `src/*/tests/*.rs` | **NO** `it_` prefix | `tests/validation.rs` |
| **In-tree Integration** | `#[cfg(test)] mod tests` | **YES** `it_` prefix | `mod it_pointer_buffer` |
| **In-tree Integration** | `src/*/tests/*.rs` | **YES** `it_` prefix | `tests/it_session.rs` |
| **Public API Integration** | `tests/*.rs` | **YES** `it_` prefix | `tests/it_api_session.rs` |

The prefix is what makes test selection possible:

- Targeted execution: `cargo test tests::it_` runs the integration tests, and `-- --skip 'tests::it_'` runs
  everything else. Cargo's filter is a substring match on the full test path, so `cargo test tests::` selects
  the `it_` tests too — use the skip form to exclude them
- Test output carries the module path, so failures name the tier

**Violating this convention breaks test filtering**, which shows up as host runs that try to issue supervisor calls,
CI failures with no console attached, and unexplained local test failures.

## Module Structure Within cfg(test)

For fewer than ~10 tests, a flat list of test functions inside `#[cfg(test)] mod tests` is sufficient. Once a module reaches 10+ tests, group them into nested `mod` blocks by concern so failure paths such as `tests::validation::validate_input_with_empty_string_fails` name the broken area, and so each concern can hold its own test utilities.

```rust
// ✅ Good — concerns are grouped, so a failing path names the area that broke
#[cfg(test)]
mod tests {
    use super::*;

    mod constructors {
        use super::*;

        #[test]
        fn new_with_aligned_range_succeeds() { /* ... */ }

        #[test]
        fn new_with_unaligned_range_fails() { /* ... */ }
    }

    mod validation {
        use super::*;

        #[test]
        fn validate_input_with_valid_data_succeeds() { /* ... */ }

        #[test]
        fn validate_input_with_invalid_data_fails() { /* ... */ }
    }

    mod it_kernel_operations {  // ✅ Good — real-environment group carries the it_ prefix
        use super::*;
        use crate::temp::temp_reservation;

        #[test]
        fn map_and_unmap_pages_work_end_to_end() { /* ... */ }
    }
}
```

## Progressive Test Complexity

Order tests within a module from simple to complex, so the right test is easy to find when debugging:

1. **Basic functionality** — happy path with minimal setup
2. **With configuration** — custom options and parameters
3. **Error scenarios** — invalid inputs, boundary cases
4. **Real environment** — supervisor calls, service sessions, mapped memory
5. **Full integration** — complete workflows, multiple resources

```rust
// ✅ Good — simple cases come first, so the first failure is the cheapest one to debug
#[cfg(test)]
mod tests {
    use super::*;

    mod feature_progression {
        use super::*;

        // 1. Basic functionality
        #[test]
        fn reserve_range_with_defaults_succeeds() { /* ... */ }

        // 2. With configuration
        #[test]
        fn reserve_range_with_custom_alignment_succeeds() { /* ... */ }

        // 3. Error scenarios
        #[test]
        fn reserve_range_with_zero_length_fails() { /* ... */ }
    }

    mod it_feature_progression {
        use super::*;
        use crate::temp::temp_reservation;

        // 4. Real environment
        #[test]
        fn map_pages_with_valid_reservation_succeeds() { /* ... */ }

        // 5. Full integration
        #[test]
        fn reserve_map_and_release_workflow_succeeds() { /* ... */ }
    }
}
```

## File Naming Rules

| Test Type | File Location | Filename Pattern | Example |
|-----------|---------------|------------------|---------|
| **Co-located unit** | Same as source | `*.rs` with `#[cfg(test)]` | `src/service_name.rs` |
| **In-tree unit** | `src/*/tests/` | No `it_` prefix | `src/session/tests/validation.rs` |
| **In-tree integration** | `src/*/tests/` | `it_*.rs` prefix | `src/session/tests/it_session.rs` |
| **Public API integration** | `tests/` (crate root) | `it_*.rs` prefix | `tests/it_api_session.rs` |

The `it_` prefix on filenames in `src/*/tests/` and `tests/` is **MANDATORY** for integration tests.

## Checklist

Before creating or moving test files, verify:

- [ ] Unit tests (pure logic, host-runnable) are co-located or in `src/*/tests/` without `it_` prefix
- [ ] In-tree integration tests (need a live kernel or console) use `it_` prefix in module or filename
- [ ] Public API integration tests are in `tests/` directory with `it_*.rs` naming
- [ ] All tests use `#[cfg(test)]` module structure when co-located
- [ ] Module names accurately reflect whether tests need the real environment
- [ ] Test file location matches test type (unit vs integration)

## References

- [test-functions](test-functions.md) - Related: Naming, Given-When-Then structure, and assertions inside a test function
- [test-organization](test-organization.md) - Related: Test tier selection (unit, integration, e2e) and nextest profiles
