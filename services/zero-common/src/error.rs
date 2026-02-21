//! Error types for the Zero ecosystem.

use thiserror::Error;

/// Result type alias using the Zero error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for Zero services.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Authorization error
    #[error("Authorization error: {0}")]
    Forbidden(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid input or request
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    /// Quota exceeded
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    /// External service error
    #[error("External service error: {0}")]
    External(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Channel send error
    #[error("Channel send error")]
    ChannelSend,

    /// Channel receive error
    #[error("Channel receive error")]
    ChannelRecv,

    /// Timeout error
    #[error("Operation timed out")]
    Timeout,

    /// Other error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<Error>,
    },
}

impl Error {
    /// Create an error with additional context.
    pub fn with_context(self, context: impl Into<String>) -> Self {
        Self::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Check if this is an authentication error.
    pub const fn is_auth(&self) -> bool {
        matches!(self, Self::Auth(_))
    }

    /// Check if this is a rate limit error.
    pub const fn is_rate_limited(&self) -> bool {
        matches!(self, Self::RateLimited(_))
    }

    /// Check if this is a quota error.
    pub const fn is_quota_exceeded(&self) -> bool {
        matches!(self, Self::QuotaExceeded(_))
    }

    /// Get HTTP status code for this error.
    pub const fn status_code(&self) -> u16 {
        match self {
            Self::Auth(_) => 401,
            Self::Forbidden(_) => 403,
            Self::NotFound(_) => 404,
            Self::InvalidInput(_) => 400,
            Self::RateLimited(_) | Self::QuotaExceeded(_) => 429,
            Self::Timeout => 408,
            Self::WithContext { source, .. } => source.status_code(),
            _ => 500,
        }
    }
}

/// Extension trait for adding context to any error type.
pub trait ResultExt<T> {
    /// Add context to an error.
    fn context(self, context: impl Into<String>) -> Result<T>;
}

impl<T, E: Into<Error>> ResultExt<T> for std::result::Result<T, E> {
    fn context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| e.into().with_context(context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(Error::Auth("test".into()).status_code(), 401);
        assert_eq!(Error::Forbidden("test".into()).status_code(), 403);
        assert_eq!(Error::NotFound("test".into()).status_code(), 404);
        assert_eq!(Error::InvalidInput("test".into()).status_code(), 400);
        assert_eq!(Error::RateLimited("test".into()).status_code(), 429);
        assert_eq!(Error::Internal("test".into()).status_code(), 500);
    }

    #[test]
    fn test_error_with_context() {
        let err = Error::Internal("db failed".into());
        let with_ctx = err.with_context("loading user");
        assert!(matches!(with_ctx, Error::WithContext { .. }));
        assert_eq!(with_ctx.status_code(), 500);
    }
}
