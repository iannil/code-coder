//! API pattern data types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Header value pattern.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HeaderPattern {
    /// Fixed header value.
    Fixed { value: String },
    /// Dynamic value extracted from context.
    Dynamic { source: String, key: String },
    /// Value comes from authentication.
    FromAuth,
}

/// Authentication pattern.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthPattern {
    /// Bearer token authentication.
    Bearer { token_source: String },
    /// Cookie-based authentication.
    Cookie { names: Vec<String> },
    /// API key in header.
    ApiKey { header: String, key_source: String },
}

/// Learned API pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPattern {
    /// Unique identifier: {host}:{method}:{path_pattern}
    pub id: String,
    /// Source host.
    pub host: String,
    /// HTTP method.
    pub method: String,
    /// Path pattern with placeholders (e.g., /users/{id}).
    pub path_pattern: String,
    /// Required headers.
    #[serde(default)]
    pub required_headers: HashMap<String, HeaderPattern>,
    /// Authentication pattern.
    pub auth: Option<AuthPattern>,
    /// Request body JSON schema.
    pub request_schema: Option<serde_json::Value>,
    /// Response body JSON schema.
    pub response_schema: Option<serde_json::Value>,
    /// When this pattern was learned.
    pub learned_at: DateTime<Utc>,
    /// Number of times this pattern was used.
    #[serde(default)]
    pub usage_count: u32,
    /// Last successful replay.
    pub last_success: Option<DateTime<Utc>>,
}

impl ApiPattern {
    /// Create a new API pattern.
    pub fn new(host: &str, method: &str, path_pattern: &str) -> Self {
        let id = format!("{}:{}:{}", host, method, path_pattern);
        Self {
            id,
            host: host.to_string(),
            method: method.to_string(),
            path_pattern: path_pattern.to_string(),
            required_headers: HashMap::new(),
            auth: None,
            request_schema: None,
            response_schema: None,
            learned_at: Utc::now(),
            usage_count: 0,
            last_success: None,
        }
    }

    /// Record a successful use of this pattern.
    pub fn record_success(&mut self) {
        self.usage_count += 1;
        self.last_success = Some(Utc::now());
    }
}

/// Raw network request snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSnapshot {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timestamp: DateTime<Utc>,
}

/// Raw network response snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSnapshot {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_pattern_new() {
        let pattern = ApiPattern::new("api.example.com", "GET", "/users/{id}");

        assert_eq!(pattern.id, "api.example.com:GET:/users/{id}");
        assert_eq!(pattern.host, "api.example.com");
        assert_eq!(pattern.method, "GET");
        assert_eq!(pattern.usage_count, 0);
    }

    #[test]
    fn test_api_pattern_record_success() {
        let mut pattern = ApiPattern::new("api.example.com", "GET", "/users");
        assert!(pattern.last_success.is_none());

        pattern.record_success();

        assert_eq!(pattern.usage_count, 1);
        assert!(pattern.last_success.is_some());
    }

    #[test]
    fn test_header_pattern_serialization() {
        let fixed = HeaderPattern::Fixed {
            value: "application/json".to_string(),
        };
        let json = serde_json::to_string(&fixed).unwrap();

        assert!(json.contains("fixed"));
        assert!(json.contains("application/json"));
    }

    #[test]
    fn test_auth_pattern_serialization() {
        let bearer = AuthPattern::Bearer {
            token_source: "env:API_TOKEN".to_string(),
        };
        let json = serde_json::to_string(&bearer).unwrap();

        assert!(json.contains("bearer"));
        assert!(json.contains("env:API_TOKEN"));
    }
}
