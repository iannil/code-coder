//! Error types for the browser service.

use thiserror::Error;

/// Browser service errors.
#[derive(Error, Debug)]
pub enum BrowserError {
    /// Browser connection failed.
    #[error("Browser connection failed: {0}")]
    ConnectionFailed(String),

    /// Session not found.
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Navigation failed.
    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    /// CDP protocol error.
    #[error("CDP error: {0}")]
    CdpError(String),

    /// Pattern extraction failed.
    #[error("Pattern extraction failed: {0}")]
    PatternExtractionFailed(String),

    /// Replay failed.
    #[error("API replay failed: {0}")]
    ReplayFailed(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
