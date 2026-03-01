# Zero Services Testing Guide

This document describes the testing strategy, structure, and commands for Zero Rust services.

## Test Structure

```
services/
├── zero-common/
│   └── src/
│       └── testing.rs     # Shared test utilities
├── zero-gateway/
│   └── src/
│       └── tests/         # Integration tests (to be created)
├── zero-channels/
│   └── src/
│       └── tests/
├── ...
```

## Test Commands

### Running Tests

```bash
# From services/ directory
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p zero-common
cargo test -p zero-gateway
cargo test -p zero-channels

# Run tests with all features
cargo test --all-features

# Run tests with specific features
cargo test -p zero-common --features="redis-backend"

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests matching pattern
cargo test pattern
```

### Code Coverage

```bash
# Install cargo-llvm-cov (one time)
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --all-features

# Generate HTML report
cargo llvm-cov --all-features --html

# Generate lcov report
cargo llvm-cov --all-features --lcov --output-path lcov.info
```

### Benchmarks

```bash
# Install criterion (if using)
# Benchmarks are in benches/ directories

# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench bench_name
```

## Test Utilities

### Using TestConfig

```rust
use zero_common::testing::TestConfig;

#[test]
fn test_with_temp_dir() {
    let config = TestConfig::new().unwrap();

    // Create files in temp directory
    config.create_file("test.txt", "content").unwrap();

    // Check file exists
    assert!(config.file_exists("test.txt"));

    // Read file
    let content = config.read_file("test.txt").unwrap();
    assert_eq!(content, "content");

    // Temp directory is automatically cleaned up when dropped
}
```

### Using Fixtures

```rust
use zero_common::testing::fixtures;

#[test]
fn test_with_fixtures() {
    let session_id = fixtures::random_session_id();
    let user_id = fixtures::random_user_id();
    let api_key = fixtures::random_api_key();

    assert!(session_id.starts_with("session-"));
    assert!(user_id.starts_with("user-"));
}
```

### Async Tests

```rust
use zero_common::async_test;

async_test!(test_async_operation, {
    let result = some_async_function().await;
    assert!(result.is_ok());
});
```

### Mock Services

```rust
use zero_common::testing::mock::CallRecorder;

#[test]
fn test_call_recording() {
    let recorder = CallRecorder::<String>::new();

    recorder.record("call1".to_string());
    recorder.record("call2".to_string());

    assert_eq!(recorder.call_count(), 2);
    assert_eq!(recorder.last_call(), Some("call2".to_string()));
}
```

### Assertions

```rust
use zero_common::{assert_ok, assert_err};
use zero_common::testing::assertions::{assert_json_eq, assert_contains};

#[test]
fn test_assertions() {
    // Result assertions
    let ok: Result<i32, &str> = Ok(42);
    let val = assert_ok!(ok);
    assert_eq!(val, 42);

    let err: Result<i32, &str> = Err("error");
    let e = assert_err!(err);
    assert_eq!(e, "error");

    // JSON assertions
    let a = serde_json::json!({"a": 1, "b": 2});
    let b = serde_json::json!({"b": 2, "a": 1});
    assert_json_eq(&a, &b); // Passes - order independent

    // String assertions
    assert_contains("hello world", "world");
}
```

## Test Categories

### Unit Tests

Located inline with source code using `#[cfg(test)]` modules:

```rust
// In src/some_module.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

### Integration Tests

Located in `tests/` directories:

```rust
// In tests/integration.rs
use zero_common::testing::TestConfig;

#[test]
fn test_full_flow() {
    let config = TestConfig::new().unwrap();
    // Test complete workflow
}
```

### Property-Based Tests

Using proptest:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_property(s in "\\PC*") {
        // Test that property holds for all inputs
        assert!(some_function(&s).is_valid());
    }
}
```

## Coverage Requirements

| Metric | Target |
|--------|--------|
| Line Coverage | 75% |
| Branch Coverage | 70% |
| Function Coverage | 80% |

## CI Integration

Tests are run automatically via GitHub Actions:

1. `cargo check --all-features` - Compile check
2. `cargo clippy --all-features` - Lint check
3. `cargo fmt --check` - Format check
4. `cargo test --all-features` - Run all tests

## Best Practices

1. **Test Isolation** - Each test should be independent
2. **Use TestConfig** - Use the provided test utilities for temp files
3. **Async Helpers** - Use `async_test!` macro for async tests
4. **Descriptive Names** - Test names should describe behavior
5. **Mock External Services** - Never call real APIs in tests
6. **Clean Up** - Use RAII (TestConfig) for automatic cleanup
7. **Property Testing** - Use proptest for complex invariants
