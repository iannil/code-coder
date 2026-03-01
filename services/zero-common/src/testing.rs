//! Test utilities for Zero ecosystem
//!
//! This module provides common testing utilities used across all Zero services:
//! - Mock configuration
//! - Test fixtures
//! - Async test helpers
//! - HTTP test utilities

use std::path::PathBuf;
use tempfile::TempDir;
use std::sync::Arc;

/// Test configuration builder
pub struct TestConfig {
    temp_dir: TempDir,
    config_path: PathBuf,
}

impl TestConfig {
    /// Create a new test configuration with isolated temp directory
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("config.json");

        // Write default minimal config
        std::fs::write(&config_path, "{}")?;

        Ok(Self {
            temp_dir,
            config_path,
        })
    }

    /// Create with custom config content
    pub fn with_config(config: serde_json::Value) -> anyhow::Result<Self> {
        let test_config = Self::new()?;
        std::fs::write(&test_config.config_path, serde_json::to_string_pretty(&config)?)?;
        Ok(test_config)
    }

    /// Get the config file path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Get the temp directory path
    pub fn temp_dir(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Create a file in the temp directory
    pub fn create_file(&self, relative_path: &str, content: &str) -> anyhow::Result<PathBuf> {
        let path = self.temp_dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Check if a file exists in the temp directory
    pub fn file_exists(&self, relative_path: &str) -> bool {
        self.temp_dir.path().join(relative_path).exists()
    }

    /// Read a file from the temp directory
    pub fn read_file(&self, relative_path: &str) -> anyhow::Result<String> {
        Ok(std::fs::read_to_string(self.temp_dir.path().join(relative_path))?)
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new().expect("Failed to create test config")
    }
}

/// Mock HTTP client for testing HTTP requests
#[cfg(feature = "hitl-client")]
pub struct MockHttpClient {
    responses: std::collections::HashMap<String, serde_json::Value>,
}

#[cfg(feature = "hitl-client")]
impl MockHttpClient {
    pub fn new() -> Self {
        Self {
            responses: std::collections::HashMap::new(),
        }
    }

    pub fn add_response(&mut self, path: &str, response: serde_json::Value) {
        self.responses.insert(path.to_string(), response);
    }

    pub fn get_response(&self, path: &str) -> Option<&serde_json::Value> {
        self.responses.get(path)
    }
}

/// Test context for async tests
pub struct TestContext {
    pub config: TestConfig,
    pub runtime: Option<tokio::runtime::Runtime>,
}

impl TestContext {
    /// Create a new test context with a runtime
    pub fn new() -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        Ok(Self {
            config: TestConfig::new()?,
            runtime: Some(runtime),
        })
    }

    /// Run an async block in the test context
    pub fn block_on<F: std::future::Future>(&self, f: F) -> F::Output {
        self.runtime.as_ref().unwrap().block_on(f)
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new().expect("Failed to create test context")
    }
}

/// Helper macro for async tests
#[macro_export]
macro_rules! async_test {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async { $body });
        }
    };
}

/// Assert that a result is Ok
#[macro_export]
macro_rules! assert_ok {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    };
    ($expr:expr, $msg:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    };
}

/// Assert that a result is Err
#[macro_export]
macro_rules! assert_err {
    ($expr:expr) => {
        match $expr {
            Ok(val) => panic!("Expected Err, got Ok: {:?}", val),
            Err(e) => e,
        }
    };
    ($expr:expr, $msg:expr) => {
        match $expr {
            Ok(val) => panic!("{}: got Ok: {:?}", $msg, val),
            Err(e) => e,
        }
    };
}

/// Fixture generator for test data
pub mod fixtures {
    use super::*;
    use uuid::Uuid;
    use chrono::{DateTime, Utc};

    /// Generate a random UUID
    pub fn random_uuid() -> String {
        Uuid::new_v4().to_string()
    }

    /// Generate a random session ID
    pub fn random_session_id() -> String {
        format!("session-{}", random_uuid())
    }

    /// Generate a random message ID
    pub fn random_message_id() -> String {
        format!("msg-{}", random_uuid())
    }

    /// Generate a random user ID
    pub fn random_user_id() -> String {
        format!("user-{}", random_uuid())
    }

    /// Generate a timestamp
    pub fn now_timestamp() -> DateTime<Utc> {
        Utc::now()
    }

    /// Generate a random API key (test only)
    pub fn random_api_key() -> String {
        format!("test-key-{}", random_uuid())
    }

    /// Generate random bytes
    pub fn random_bytes(len: usize) -> Vec<u8> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..len).map(|_| rng.gen::<u8>()).collect()
    }
}

/// Test assertions
pub mod assertions {
    /// Assert that two JSON values are equal (ignoring order for objects and arrays)
    pub fn assert_json_eq(expected: &serde_json::Value, actual: &serde_json::Value) {
        match (expected, actual) {
            (serde_json::Value::Object(e), serde_json::Value::Object(a)) => {
                assert_eq!(e.len(), a.len(), "Object length mismatch");
                for (key, value) in e {
                    let actual_value = a.get(key).unwrap_or_else(|| panic!("Missing key: {}", key));
                    assert_json_eq(value, actual_value);
                }
            }
            (serde_json::Value::Array(e), serde_json::Value::Array(a)) => {
                assert_eq!(e.len(), a.len(), "Array length mismatch");
                for (i, (ev, av)) in e.iter().zip(a.iter()).enumerate() {
                    assert_json_eq(ev, av);
                }
            }
            _ => assert_eq!(expected, actual),
        }
    }

    /// Assert that a string contains a substring
    pub fn assert_contains(haystack: &str, needle: &str) {
        assert!(
            haystack.contains(needle),
            "Expected '{}' to contain '{}'",
            haystack,
            needle
        );
    }

    /// Assert that a string does not contain a substring
    pub fn assert_not_contains(haystack: &str, needle: &str) {
        assert!(
            !haystack.contains(needle),
            "Expected '{}' to not contain '{}'",
            haystack,
            needle
        );
    }
}

/// Mock service for testing
pub mod mock {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Call recorder for tracking function calls
    pub struct CallRecorder<T: Clone> {
        calls: Mutex<Vec<T>>,
        call_count: AtomicUsize,
    }

    impl<T: Clone> CallRecorder<T> {
        pub fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                call_count: AtomicUsize::new(0),
            }
        }

        pub fn record(&self, call: T) {
            self.calls.lock().unwrap().push(call);
            self.call_count.fetch_add(1, Ordering::SeqCst);
        }

        pub fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }

        pub fn calls(&self) -> Vec<T> {
            self.calls.lock().unwrap().clone()
        }

        pub fn last_call(&self) -> Option<T> {
            self.calls.lock().unwrap().last().cloned()
        }

        pub fn reset(&self) {
            self.calls.lock().unwrap().clear();
            self.call_count.store(0, Ordering::SeqCst);
        }
    }

    impl<T: Clone> Default for CallRecorder<T> {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = TestConfig::new().unwrap();
        assert!(config.temp_dir().exists());
        assert!(config.config_path().exists());
    }

    #[test]
    fn test_config_with_custom_content() {
        let custom = serde_json::json!({
            "key": "value",
            "nested": { "a": 1 }
        });
        let config = TestConfig::with_config(custom.clone()).unwrap();

        let content: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(config.config_path()).unwrap()
        ).unwrap();

        assert_eq!(content["key"], "value");
        assert_eq!(content["nested"]["a"], 1);
    }

    #[test]
    fn test_create_file() {
        let config = TestConfig::new().unwrap();
        let path = config.create_file("subdir/test.txt", "hello").unwrap();

        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
        assert!(config.file_exists("subdir/test.txt"));
    }

    #[test]
    fn test_fixtures() {
        let uuid = fixtures::random_uuid();
        assert!(!uuid.is_empty());
        assert!(uuid.len() == 36); // UUID v4 format

        let session_id = fixtures::random_session_id();
        assert!(session_id.starts_with("session-"));
    }

    #[test]
    fn test_call_recorder() {
        let recorder = mock::CallRecorder::<String>::new();

        recorder.record("call1".to_string());
        recorder.record("call2".to_string());

        assert_eq!(recorder.call_count(), 2);
        assert_eq!(recorder.last_call(), Some("call2".to_string()));

        recorder.reset();
        assert_eq!(recorder.call_count(), 0);
    }

    #[test]
    fn test_json_assertions() {
        let a = serde_json::json!({"a": 1, "b": 2});
        let b = serde_json::json!({"b": 2, "a": 1});
        assertions::assert_json_eq(&a, &b);
    }
}
