//! NAPI bindings for Web technology fingerprint detection
//!
//! Provides JavaScript/TypeScript bindings for:
//! - WebFingerprintEngineHandle: Web technology detection engine
//! - Utility functions: detectWebTechnologies

use std::collections::HashMap;
use std::sync::Arc;

use napi_derive::napi;

use crate::web::{
    WebCategory, WebDetection as RustWebDetection,
    WebFingerprintEngine as RustWebFingerprintEngine,
    WebFingerprintInput as RustWebFingerprintInput, WebFingerprint as RustWebFingerprint,
    WEB_FINGERPRINT_ENGINE,
};

// ============================================================================
// NAPI Types
// ============================================================================

/// Technology detection result
#[napi(object)]
pub struct NapiWebDetection {
    pub name: String,
    pub category: String,
    pub website: Option<String>,
    pub matches: Vec<String>,
    pub confidence: String,
}

impl From<RustWebDetection> for NapiWebDetection {
    fn from(d: RustWebDetection) -> Self {
        Self {
            name: d.name,
            category: d.category,
            website: d.website,
            matches: d.matches,
            confidence: d.confidence,
        }
    }
}

/// Web fingerprint info
#[napi(object)]
pub struct NapiWebFingerprint {
    pub name: String,
    pub category: String,
    pub website: Option<String>,
    pub pattern_count: u32,
}

impl From<&RustWebFingerprint> for NapiWebFingerprint {
    fn from(fp: &RustWebFingerprint) -> Self {
        Self {
            name: fp.name.clone(),
            category: fp.category.as_str().to_string(),
            website: fp.website.clone(),
            pattern_count: fp.patterns.len() as u32,
        }
    }
}

/// Input for web fingerprint detection
#[napi(object)]
pub struct NapiWebFingerprintInput {
    /// HTML/JS content to analyze
    pub content: Option<String>,
    /// HTTP headers (header_name -> value)
    pub headers: Option<HashMap<String, String>>,
    /// URL being analyzed
    pub url: Option<String>,
    /// Cookie names
    pub cookies: Option<Vec<String>>,
}

impl From<NapiWebFingerprintInput> for RustWebFingerprintInput {
    fn from(i: NapiWebFingerprintInput) -> Self {
        Self {
            content: i.content.unwrap_or_default(),
            headers: i.headers.unwrap_or_default(),
            url: i.url,
            cookies: i.cookies.unwrap_or_default(),
        }
    }
}

// ============================================================================
// WebFingerprintEngineHandle
// ============================================================================

/// Thread-safe web fingerprint engine handle
#[napi]
pub struct WebFingerprintEngineHandle {
    inner: Arc<RustWebFingerprintEngine>,
}

#[napi]
impl WebFingerprintEngineHandle {
    /// Create a new web fingerprint engine
    #[napi(factory)]
    pub fn create() -> Self {
        Self {
            inner: Arc::new(RustWebFingerprintEngine::new()),
        }
    }

    /// Detect web technologies from input
    #[napi]
    pub fn detect(&self, input: NapiWebFingerprintInput) -> Vec<NapiWebDetection> {
        let rust_input: RustWebFingerprintInput = input.into();
        self.inner
            .detect(&rust_input)
            .into_iter()
            .map(|d| d.into())
            .collect()
    }

    /// Get all fingerprint definitions
    #[napi]
    pub fn fingerprints(&self) -> Vec<NapiWebFingerprint> {
        self.inner
            .fingerprints()
            .iter()
            .map(|fp| fp.into())
            .collect()
    }

    /// Get fingerprints by category
    #[napi]
    pub fn fingerprints_by_category(&self, category: String) -> Vec<NapiWebFingerprint> {
        let cat = match WebCategory::from_str(&category) {
            Some(c) => c,
            None => return Vec::new(),
        };

        self.inner
            .fingerprints_by_category(cat)
            .into_iter()
            .map(|fp| fp.into())
            .collect()
    }

    /// Get all category names
    #[napi]
    pub fn categories(&self) -> Vec<String> {
        self.inner
            .categories()
            .into_iter()
            .map(|c| c.as_str().to_string())
            .collect()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Detect web technologies using the global fingerprint engine
#[napi]
pub fn detect_web_technologies(input: NapiWebFingerprintInput) -> Vec<NapiWebDetection> {
    let rust_input: RustWebFingerprintInput = input.into();
    WEB_FINGERPRINT_ENGINE
        .detect(&rust_input)
        .into_iter()
        .map(|d| d.into())
        .collect()
}

/// Get all available web technology fingerprints
#[napi]
pub fn get_web_fingerprints() -> Vec<NapiWebFingerprint> {
    WEB_FINGERPRINT_ENGINE
        .fingerprints()
        .iter()
        .map(|fp| fp.into())
        .collect()
}

/// Get web fingerprints by category
#[napi]
pub fn get_web_fingerprints_by_category(category: String) -> Vec<NapiWebFingerprint> {
    let cat = match WebCategory::from_str(&category) {
        Some(c) => c,
        None => return Vec::new(),
    };

    WEB_FINGERPRINT_ENGINE
        .fingerprints_by_category(cat)
        .into_iter()
        .map(|fp| fp.into())
        .collect()
}

/// Get all web technology categories
#[napi]
pub fn get_web_categories() -> Vec<String> {
    WEB_FINGERPRINT_ENGINE
        .categories()
        .into_iter()
        .map(|c| c.as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_fingerprint_engine_handle() {
        let engine = WebFingerprintEngineHandle::create();
        let categories = engine.categories();
        assert!(!categories.is_empty());
        assert!(categories.contains(&"frontend".to_string()));
        assert!(categories.contains(&"ui".to_string()));
        assert!(categories.contains(&"analytics".to_string()));
    }

    #[test]
    fn test_detect_web_technologies() {
        let input = NapiWebFingerprintInput {
            content: Some("data-reactroot __NEXT_DATA__".to_string()),
            headers: None,
            url: None,
            cookies: None,
        };

        let detections = detect_web_technologies(input);
        assert!(detections.iter().any(|d| d.name == "React"));
        assert!(detections.iter().any(|d| d.name == "Next.js"));
    }

    #[test]
    fn test_fingerprints_by_category() {
        let engine = WebFingerprintEngineHandle::create();
        let frameworks = engine.fingerprints_by_category("frontend".to_string());
        assert!(!frameworks.is_empty());
        assert!(frameworks.iter().any(|fp| fp.name == "React"));
        assert!(frameworks.iter().any(|fp| fp.name == "Vue"));
    }

    #[test]
    fn test_detect_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("x-vercel-id".to_string(), "iad1::12345".to_string());

        let input = NapiWebFingerprintInput {
            content: None,
            headers: Some(headers),
            url: None,
            cookies: None,
        };

        let detections = detect_web_technologies(input);
        assert!(detections.iter().any(|d| d.name == "Vercel"));
    }

    #[test]
    fn test_get_web_fingerprints() {
        let fps = get_web_fingerprints();
        assert!(!fps.is_empty());
        assert!(fps.iter().any(|fp| fp.name == "React"));
        assert!(fps.iter().any(|fp| fp.name == "Tailwind CSS"));
    }

    #[test]
    fn test_get_web_categories() {
        let cats = get_web_categories();
        assert!(cats.contains(&"frontend".to_string()));
        assert!(cats.contains(&"ui".to_string()));
        assert!(cats.contains(&"state".to_string()));
        assert!(cats.contains(&"build".to_string()));
        assert!(cats.contains(&"analytics".to_string()));
        assert!(cats.contains(&"auth".to_string()));
        assert!(cats.contains(&"payment".to_string()));
    }
}
