//! Speech-to-text (STT) module for voice message transcription.
//!
//! This module provides a trait-based abstraction for STT services,
//! with implementations for OpenAI Whisper and OpenAI-compatible providers.

mod compatible;
mod openai;
mod traits;

pub use compatible::CompatibleStt;
pub use openai::OpenAiStt;
pub use traits::SpeechToText;

use std::sync::Arc;

/// Create an STT provider based on the provider name.
///
/// # Arguments
/// * `provider` - Provider name: "openai", "uniapi", or other OpenAI-compatible providers
/// * `api_key` - API key for the provider
/// * `model` - Optional model name
/// * `base_url` - Optional base URL for OpenAI-compatible providers
///
/// # Returns
/// An STT implementation, or an error if the provider is not supported.
pub fn create_stt(
    provider: &str,
    api_key: &str,
    model: Option<&str>,
    base_url: Option<&str>,
) -> anyhow::Result<Arc<dyn SpeechToText>> {
    match provider.to_lowercase().as_str() {
        "openai" | "whisper" => Ok(Arc::new(OpenAiStt::new(
            api_key.to_string(),
            model.map(ToString::to_string),
        ))),
        // OpenAI-compatible providers with known base URLs
        "uniapi" => Ok(Arc::new(CompatibleStt::new(
            api_key.to_string(),
            base_url.unwrap_or("https://hk.uniapi.io"),
            model.map(ToString::to_string),
        ))),
        "groq" => Ok(Arc::new(CompatibleStt::new(
            api_key.to_string(),
            base_url.unwrap_or("https://api.groq.com/openai"),
            model.map(ToString::to_string),
        ))),
        "deepinfra" => Ok(Arc::new(CompatibleStt::new(
            api_key.to_string(),
            base_url.unwrap_or("https://api.deepinfra.com"),
            model.map(ToString::to_string),
        ))),
        // Generic OpenAI-compatible provider (requires base_url)
        "compatible" | "openai-compatible" => {
            let url = base_url.ok_or_else(|| {
                anyhow::anyhow!("base_url is required for 'compatible' STT provider")
            })?;
            Ok(Arc::new(CompatibleStt::new(
                api_key.to_string(),
                url,
                model.map(ToString::to_string),
            )))
        }
        // Local Whisper server (faster-whisper-server, whisper.cpp, etc.)
        // No API key required - runs on localhost
        "local" | "faster-whisper" | "whisper-local" => {
            let url = base_url.ok_or_else(|| {
                anyhow::anyhow!(
                    "base_url is required for 'local' STT provider (e.g., http://localhost:8000)"
                )
            })?;
            Ok(Arc::new(CompatibleStt::new(
                api_key.to_string(), // Can be empty for local services
                url,
                model.map(ToString::to_string),
            )))
        }
        _ => anyhow::bail!(
            "Unsupported STT provider: {provider}. Supported: openai, uniapi, groq, deepinfra, compatible, local"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_openai_stt() {
        let stt = create_stt("openai", "sk-test", None, None);
        assert!(stt.is_ok());
    }

    #[test]
    fn create_whisper_alias() {
        let stt = create_stt("whisper", "sk-test", Some("whisper-1"), None);
        assert!(stt.is_ok());
    }

    #[test]
    fn create_uniapi_stt() {
        let stt = create_stt("uniapi", "sk-test", Some("qwen3-asr-flash"), None);
        assert!(stt.is_ok());
    }

    #[test]
    fn create_groq_stt() {
        let stt = create_stt("groq", "sk-test", Some("whisper-large-v3"), None);
        assert!(stt.is_ok());
    }

    #[test]
    fn create_compatible_requires_base_url() {
        let stt = create_stt("compatible", "sk-test", None, None);
        assert!(stt.is_err());

        let stt = create_stt("compatible", "sk-test", None, Some("https://api.example.com"));
        assert!(stt.is_ok());
    }

    #[test]
    fn create_unsupported_fails() {
        let stt = create_stt("unsupported", "key", None, None);
        assert!(stt.is_err());
        let err = stt.err().expect("expected error");
        assert!(err.to_string().contains("Unsupported"));
    }

    #[test]
    fn create_case_insensitive() {
        assert!(create_stt("OpenAI", "key", None, None).is_ok());
        assert!(create_stt("OPENAI", "key", None, None).is_ok());
        assert!(create_stt("UniAPI", "key", None, None).is_ok());
    }

    #[test]
    fn create_local_stt_requires_base_url() {
        // local provider requires base_url
        let stt = create_stt("local", "", None, None);
        assert!(stt.is_err());
        let err = stt.err().expect("expected error");
        assert!(err.to_string().contains("base_url is required"));
    }

    #[test]
    fn create_local_stt_with_base_url() {
        // local provider works with base_url and empty api_key
        let stt = create_stt("local", "", Some("base"), Some("http://localhost:8000"));
        assert!(stt.is_ok());
    }

    #[test]
    fn create_faster_whisper_alias() {
        // faster-whisper alias for local provider
        let stt = create_stt(
            "faster-whisper",
            "",
            Some("small"),
            Some("http://localhost:8000"),
        );
        assert!(stt.is_ok());
    }

    #[test]
    fn create_whisper_local_alias() {
        // whisper-local alias for local provider
        let stt = create_stt(
            "whisper-local",
            "",
            Some("medium"),
            Some("http://localhost:8000"),
        );
        assert!(stt.is_ok());
    }
}
