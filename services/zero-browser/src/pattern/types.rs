//! API pattern types.

use serde::{Deserialize, Serialize};

/// An extracted API pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiPattern {
    /// Unique identifier for the pattern.
    pub id: String,
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// URL pattern with placeholders.
    pub url_pattern: String,
    /// Header patterns.
    pub headers: Vec<HeaderPattern>,
    /// Authentication pattern if detected.
    pub auth: Option<AuthPattern>,
    /// Request body schema if applicable.
    pub body_schema: Option<String>,
    /// When this pattern was first observed.
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// Number of times this pattern was observed.
    pub occurrence_count: u32,
}

/// A header pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderPattern {
    /// Header name.
    pub name: String,
    /// Whether this header is required.
    pub required: bool,
    /// Whether the value is dynamic (changes between requests).
    pub dynamic: bool,
    /// Static value if not dynamic.
    pub static_value: Option<String>,
}

/// Authentication pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthPattern {
    /// Bearer token authentication.
    Bearer {
        /// Header name (usually "Authorization").
        header: String,
    },
    /// API key authentication.
    ApiKey {
        /// Header name.
        header: String,
    },
    /// Cookie-based authentication.
    Cookie {
        /// Cookie names used for auth.
        cookie_names: Vec<String>,
    },
    /// Custom authentication.
    Custom {
        /// Description of the auth mechanism.
        description: String,
    },
}
