//! Text-to-speech (TTS) module for voice synthesis.
//!
//! This module provides a trait-based abstraction for TTS services,
//! with implementations for OpenAI and ElevenLabs.

mod elevenlabs;
mod openai;
mod traits;

pub use elevenlabs::ElevenLabsTts;
pub use openai::OpenAiTts;
#[allow(unused_imports)]
pub use traits::{AudioFormat, SynthesisOptions, TextToSpeech, VoiceInfo};

use std::sync::Arc;

/// Create a TTS provider based on the provider name.
///
/// # Arguments
/// * `provider` - Provider name: "openai", "elevenlabs", or "compatible"
/// * `api_key` - API key for the provider
/// * `model` - Optional model name
/// * `voice` - Optional default voice ID
/// * `base_url` - Optional base URL for OpenAI-compatible providers
///
/// # Returns
/// A TTS implementation, or an error if the provider is not supported.
pub fn create_tts(
    provider: &str,
    api_key: &str,
    model: Option<&str>,
    voice: Option<&str>,
    base_url: Option<&str>,
) -> anyhow::Result<Arc<dyn TextToSpeech>> {
    match provider.to_lowercase().as_str() {
        "openai" => Ok(Arc::new(OpenAiTts::new(
            api_key.to_string(),
            model.map(ToString::to_string),
            voice.map(ToString::to_string),
        ))),
        "elevenlabs" | "eleven" | "11labs" => Ok(Arc::new(ElevenLabsTts::new(
            api_key.to_string(),
            model.map(ToString::to_string),
            voice.map(ToString::to_string),
        ))),
        // OpenAI-compatible providers with custom base URL
        "compatible" | "openai-compatible" => {
            let url = base_url.ok_or_else(|| {
                anyhow::anyhow!("base_url is required for 'compatible' TTS provider")
            })?;
            Ok(Arc::new(OpenAiTts::with_base_url(
                api_key.to_string(),
                url,
                model.map(ToString::to_string),
                voice.map(ToString::to_string),
            )))
        }
        _ => anyhow::bail!(
            "Unsupported TTS provider: {provider}. Supported: openai, elevenlabs, compatible"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_openai_tts() {
        let tts = create_tts("openai", "sk-test", None, None, None);
        assert!(tts.is_ok());
        assert_eq!(tts.unwrap().provider_name(), "openai");
    }

    #[test]
    fn create_elevenlabs_tts() {
        let tts = create_tts("elevenlabs", "xi-test", None, None, None);
        assert!(tts.is_ok());
        assert_eq!(tts.unwrap().provider_name(), "elevenlabs");
    }

    #[test]
    fn create_eleven_alias() {
        let tts = create_tts("eleven", "xi-test", Some("eleven_turbo_v2"), None, None);
        assert!(tts.is_ok());
        assert_eq!(tts.unwrap().provider_name(), "elevenlabs");
    }

    #[test]
    fn create_11labs_alias() {
        let tts = create_tts("11labs", "xi-test", None, Some("voice-id"), None);
        assert!(tts.is_ok());
    }

    #[test]
    fn create_compatible_requires_base_url() {
        let tts = create_tts("compatible", "sk-test", None, None, None);
        assert!(tts.is_err());

        let tts = create_tts(
            "compatible",
            "sk-test",
            None,
            None,
            Some("https://api.example.com"),
        );
        assert!(tts.is_ok());
    }

    #[test]
    fn create_unsupported_fails() {
        let tts = create_tts("unsupported", "key", None, None, None);
        assert!(tts.is_err());
        let err = tts.err().expect("expected error");
        assert!(err.to_string().contains("Unsupported"));
    }

    #[test]
    fn create_case_insensitive() {
        assert!(create_tts("OpenAI", "key", None, None, None).is_ok());
        assert!(create_tts("OPENAI", "key", None, None, None).is_ok());
        assert!(create_tts("ElevenLabs", "key", None, None, None).is_ok());
    }
}
