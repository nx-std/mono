---
name: "test-functions"
description: "Test naming conventions, function structure, Given-When-Then, async tests, assertions, forbidden patterns. Load when writing or reviewing test functions"
type: "core"
scope: "global"
---

# Test Functions - Naming and Structure

**MANDATORY patterns for writing individual test functions in Rust**

## Test Naming Conventions

Naming is the first decision when writing a test. Every test function uses the format:

`<function_name>_<scenario>_<expected_outcome>()`

- **function_name**: the exact name of the function being tested
- **scenario**: the specific input condition, state, or situation
- **expected_outcome**: what should happen (`succeeds`, `fails`, `returns_none`, ...)

A name in this form answers what is tested, under what conditions, and what should happen, so a CI failure is legible without opening the test body.

```rust
// ✅ Good — each name states the scenario and the outcome, so a failing CI line is self-explanatory
#[test]
fn write_in_header_with_valid_command_id_succeeds() { /* ... */ }

#[test]
fn write_in_header_with_too_many_objects_fails() { /* ... */ }

#[test]
fn find_object_with_unknown_id_returns_none() { /* ... */ }

#[test]
fn decode_out_header_with_bad_magic_fails() { /* ... */ }

#[test]
fn parse_service_name_with_max_length_succeeds() { /* ... */ }
```

```rust
// ❌ Bad — vague names force a reader to open the body to learn what broke
#[test]
fn test_write_header() { /* ... */ }

#[test]
fn write_header_works() { /* ... */ }

// ❌ Bad — "test" in the name is redundant; #[test] already says it
#[test]
fn test_write_in_header_with_valid_command_id_succeeds() { /* ... */ }

#[test]
fn parse_service_name_test_returns_error() { /* ... */ }

// ❌ Bad — missing scenario: which input made it succeed?
#[test]
fn write_in_header_succeeds() { /* ... */ }

// ❌ Bad — missing expected outcome: succeeds or fails?
#[test]
fn write_in_header_with_valid_command_id() { /* ... */ }

// ❌ Bad — two functions under test violates single responsibility; split into two tests
#[test]
fn encode_and_decode_in_header_succeeds() { /* ... */ }
```

### Naming by Test Type

Host unit tests name input conditions and encoding rules; on-hardware tests name kernel or console state; on-hardware workflow tests name end-to-end sequences.

```rust
// ✅ Good — host unit: the scenario is an input condition
fn decode_out_header_with_bad_magic_fails() {}

// ✅ Good — on-hardware: the scenario is kernel state
fn map_shared_memory_with_already_mapped_range_fails() {}

// ✅ Good — on-hardware: the scenario is an end-to-end sequence
fn connect_service_and_dispatch_command_workflow_succeeds() {}
```

### Length and Clarity

Be descriptive but concise, use domain terminology consistently, avoid abbreviations that are not well established in the domain, and stay near ~60 characters where possible, prioritizing clarity over the limit.

```rust
// ✅ Good — descriptive and still readable at a glance
fn reserve_pages_with_exhausted_region_returns_error() {}

// 🔶 Acceptable — over-long, but only where the extra words are needed for clarity
fn decode_out_header_with_raw_size_shorter_than_trailing_pad_fails_cleanly() {}

// ❌ Bad — abbreviations that no reader can decode
fn dec_hdr_inv_mgc_fails() {}
```

## Testing Framework Selection

Use standard `#[test]` for logic that runs on the host, and place anything that needs a live kernel in the on-hardware NRO suite. A host `#[test]` may only touch pure logic — header packing, wire-format decoding, page arithmetic, bit fields, result-code decomposition. The moment a test needs a real session handle, a mapped page, or a running thread, it belongs on the console, where the harness runs it as part of the test NRO.

```rust
// ✅ Good — pure encoding logic, so a host #[test] covers it without a console
#[test]
fn pack_command_id_with_domain_request_returns_expected_words() {
    //* Given
    let request = CmifRequest::new(CommandId::new(7));

    //* When
    let words = pack_command_id(&request);

    //* Then
    assert_eq!(words, expected_words(), "packed words should match the CMIF layout");
}

// ✅ Good — needs a live session from the kernel, so it lives in the on-hardware suite
// Registered with the NRO harness; run via the /code-deploy and /code-test skills.
fn dispatch_command_with_open_session_succeeds() {
    //* Given
    let session = open_test_session();

    //* When
    let result = dispatch_command(&session, CommandId::new(0));

    //* Then
    assert!(result.is_ok(), "dispatch should succeed on an open session");
}
```

## Given-When-Then Structure (Mandatory)

Every test follows the Given-When-Then pattern with **MANDATORY** `//* Given`, `//* When`, and `//* Then` marker comments. The markers are keyword-only: no trailing text on the marker line; any prose goes on the next line as an ordinary `//` comment.

| Marker | Required | Purpose | Content |
|---|---|---|---|
| `//* Given` | Optional (omit when there is no setup) | Preconditions, test data, fixtures, system state | Variable declarations, buffer setup, fixture construction |
| `//* When` | Required | Execute **exactly one** function under test | **Only** the single call being tested |
| `//* Then` | Required | Assert outcomes and side effects | **Only** assertions and assertion helpers such as `.expect()` used to extract a value |

More than one call in `//* When` means the test scope is too broad and failure attribution is impossible. Business logic in `//* Then` obscures what is being verified: if a value must be transformed before asserting, the transformation belongs in `//* Given`, or the test is verifying two things and should be split.

```rust
// ✅ Good — one call under test, and the Then section only asserts
#[test]
fn write_in_header_with_valid_command_id_succeeds() {
    //* Given
    let mut buffer = [0u8; RAW_DATA_CAPACITY];
    let command_id = CommandId::new(3);
    let expected_magic = IN_HEADER_MAGIC;

    //* When
    let result = write_in_header(&mut buffer, command_id);

    //* Then
    assert!(result.is_ok(), "header write should succeed with a valid command id");
    let written = result.expect("should return the number of bytes written");
    assert_eq!(written, InHeader::SIZE, "should write exactly one header");
    let header = decode_in_header(&buffer)
        .expect("should decode the header that was just written");
    assert_eq!(header.magic, expected_magic, "magic should be SFCI");
    assert_eq!(header.command_id, command_id, "command id should round-trip");
}

// ✅ Good — no setup needed, so Given is omitted
#[test]
fn parse_service_name_with_empty_input_fails() {
    //* When
    let result = ServiceName::parse("");

    //* Then
    assert!(result.is_err(), "parsing should fail for an empty service name");
    let error = result.expect_err("should return a parse error");
    assert!(matches!(error, ServiceNameError::Empty),
        "Expected Empty error, got {:?}", error);
}
```

```rust
// ❌ Bad — no markers, so the reader cannot tell setup from action from assertion
#[test]
fn parse_service_name_with_valid_input_succeeds() {
    let input = "fsp-srv";
    let result = ServiceName::parse(input);
    assert!(result.is_ok());
}

// ❌ Bad — two functions in When, so a failure names neither of them
#[test]
fn reserve_pages_with_free_region_succeeds() {
    //* Given
    let mut map = fresh_reservation_map(64);

    //* When
    let region = allocate_region(&mut map, 8);
    let result = reserve_pages(&mut map, region);

    //* Then
    assert!(result.is_ok());
}

// ❌ Bad — business logic in Then hides which behavior is actually under test
#[test]
fn decode_out_header_with_valid_bytes_returns_header() {
    //* Given
    let buffer = encoded_out_header();

    //* When
    let result = decode_out_header(&buffer);

    //* Then
    assert!(result.is_ok());
    let header = result.expect("should decode header");
    let payload_words = (header.raw_size - InHeader::SIZE) / 4;
    let expected_offset = payload_words * 4 + CMIF_HEADER_ALIGN;
    assert_eq!(header.payload_offset, expected_offset);
}
```

## Forbidden Patterns

### Never Use `unwrap()` in Tests

A panicking `unwrap()` reports only "called `Result::unwrap()` on an `Err` value". `.expect("...")` turns the same panic into an actionable message naming what was expected and what happened.

```rust
// ❌ Bad — panics with no context about what was expected
let header = decode_out_header(&buffer).unwrap();

// ✅ Good — the panic message names the expectation and prints the actual error
let header = decode_out_header(&buffer)
    .expect("decode_out_header should succeed for a well-formed SFCO buffer");
```

### Never Test Multiple Functions in One Test

One test exercises exactly one function, as required by the `//* When` rule above. Two calls under test means a failure names neither of them; split the test in two.

## Assertion Patterns

Every assertion carries a descriptive failure message.

```rust
// ✅ Good — each assertion states what should hold, so failures read as sentences
fn assertions() {
    assert_eq!(actual, expected, "values should be equal");
    assert_ne!(actual, unexpected, "values should be different");
    assert!(condition, "condition should be true");
    assert!(result.is_ok(), "operation should succeed");
    assert!(result.is_err(), "operation should fail");

    // For Option types
    assert!(option.is_some(), "should contain value");
    assert!(option.is_none(), "should be empty");

    // For custom error types
    let error = result.expect_err("operation should fail with a malformed header");
    assert!(matches!(error, CmifError::BadMagic(_)),
        "Expected BadMagic, got {:?}", error);

    // For Result types
    let value = result.expect("operation should succeed with a well-formed request");
}
```

For collections, assert the shape first, then locate individual items and assert on them.

```rust
// ✅ Good — the length assertion fails first and explains a size mismatch before item lookups panic
#[test]
fn collect_handles_with_two_descriptors_returns_both() {
    //* Given
    let descriptors = HandleSet {
        entries: alloc::vec![
            HandleEntry { index: 0, raw: 0x0F00, transfer: false },
            HandleEntry { index: 1, raw: 0x0F04, transfer: true },
        ],
    };

    //* When
    let result = collect_handles(&descriptors)
        .expect("handle collection should succeed for valid descriptors");

    //* Then
    assert_eq!(result.copied.len(), 2, "should collect both descriptors");
    let first = result.copied.iter()
        .find(|h| h.index == 0)
        .expect("descriptor 0 should be in the collected results");
    assert_eq!(first.raw, 0x0F00, "raw handle value should round-trip");
    assert!(!first.transfer, "descriptor 0 should be copied, not transferred");
}
```

## Checklist

Before submitting a test function for review, verify:

- [ ] Test name follows `<function_name>_<scenario>_<expected_outcome>` format
- [ ] Test name does NOT include the word "test" (it's already marked with `#[test]`)
- [ ] Test uses the correct tier: host `#[test]` for pure logic, on-hardware NRO suite when a live kernel is needed
- [ ] Test has `//* Given`, `//* When`, and `//* Then` comments (Given optional if no setup needed)
- [ ] Marker comments are keyword-only; explanatory prose goes on a following `//` line
- [ ] `//* When` section calls EXACTLY ONE function under test
- [ ] `//* Then` section contains ONLY assertions and assertion helpers
- [ ] No `unwrap()` calls - all use `.expect("descriptive message")` instead
- [ ] All assertions have descriptive failure messages
- [ ] Test focuses on a single scenario (not testing multiple functions or workflows)
- [ ] Test name is descriptive and explains what is being tested

## References

- [test-files](test-files.md) - Related: Where test modules and files live in the directory structure
- [test-organization](test-organization.md) - Related: Test tier selection (unit, integration, e2e) and nextest profiles
