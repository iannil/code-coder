//! Web technology fingerprint detection module
//!
//! Provides high-performance detection of web technologies including:
//! - Frontend frameworks (React, Vue, Angular, Svelte, etc.)
//! - UI libraries (Tailwind, MUI, Ant Design, etc.)
//! - State management (Redux, Zustand, Pinia, etc.)
//! - Build tools (Vite, Webpack, Rollup, etc.)
//! - Analytics (Google Analytics, Plausible, PostHog, etc.)
//! - Authentication (Auth0, Clerk, NextAuth, etc.)
//!
//! # Performance
//!
//! Uses aho-corasick for O(n) multi-pattern matching, providing 5-10x
//! performance improvement over sequential String.includes() calls.
//!
//! # Example
//!
//! ```ignore
//! use zero_core::web::{WebFingerprintEngine, WebFingerprintInput};
//!
//! let engine = WebFingerprintEngine::new();
//! let detections = engine.detect(&WebFingerprintInput {
//!     content: "data-reactroot __NEXT_DATA__".to_string(),
//!     headers: Default::default(),
//! });
//!
//! for d in detections {
//!     println!("{}: {} ({:?})", d.name, d.category, d.confidence);
//! }
//! ```

pub mod fingerprint;

pub use fingerprint::{
    WebCategory, WebConfidence, WebDetection, WebFingerprint, WebFingerprintEngine,
    WebFingerprintInput, WebPatternType, WEB_FINGERPRINT_ENGINE,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_imports() {
        let _ = WebFingerprintEngine::new();
    }
}
