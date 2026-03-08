//! OpenAI TTS implementation.
//!
//! Uses the OpenAI Audio API for text-to-speech synthesis.
//! Supports tts-1 and tts-1-hd models with multiple voices.

use super::{AudioFormat, SynthesisOptions, TextToSpeech, VoiceInfo};
use async_trait::async_trait;
use reqwest::Client;

/// OpenAI TTS voices.
const OPENAI_VOICES: [(&str, &str, &str); 6] = [
    ("alloy", "Alloy", "Neutral, balanced voice"),
    ("echo", "Echo", "Warm, clear voice"),
    ("fable", "Fable", "British accent, expressive"),
    ("onyx", "Onyx", "Deep, authoritative voice"),
    ("nova", "Nova", "Friendly, upbeat voice"),
    ("shimmer", "Shimmer", "Soft, calming voice"),
];

/// OpenAI TTS API implementation.
pub struct OpenAiTts {
    api_key: String,
    client: Client,
    model: String,
    default_voice: String,
    base_url: String,
}

impl OpenAiTts {
    /// Create a new OpenAI TTS client.
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key
    /// * `model` - Model name (default: "tts-1")
    /// * `voice` - Default voice (default: "alloy")
    pub fn new(api_key: String, model: Option<String>, voice: Option<String>) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            model: model.unwrap_or_else(|| "tts-1".to_string()),
            default_voice: voice.unwrap_or_else(|| "alloy".to_string()),
            base_url: "https://api.openai.com".to_string(),
        }
    }

    /// Create with a custom base URL (for OpenAI-compatible providers).
    pub fn with_base_url(
        api_key: String,
        base_url: &str,
        model: Option<String>,
        voice: Option<String>,
    ) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            model: model.unwrap_or_else(|| "tts-1".to_string()),
            default_voice: voice.unwrap_or_else(|| "alloy".to_string()),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn format_to_response_format(format: AudioFormat) -> &'static str {
        match format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Opus | AudioFormat::Ogg => "opus",
            AudioFormat::Aac => "aac",
            AudioFormat::Flac => "flac",
            AudioFormat::Wav => "wav",
        }
    }
}

#[async_trait]
impl TextToSpeech for OpenAiTts {
    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> anyhow::Result<Vec<u8>> {
        let opts = options.unwrap_or_default();
        let voice = opts.voice.as_deref().unwrap_or(&self.default_voice);
        let format = Self::format_to_response_format(opts.format);

        let mut body = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": voice,
            "response_format": format
        });

        // Add speed if specified (OpenAI supports 0.25 - 4.0)
        if let Some(speed) = opts.speed {
            let clamped = speed.clamp(0.25, 4.0);
            body["speed"] = serde_json::json!(clamped);
        }

        let url = format!("{}/v1/audio/speech", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI TTS API error ({status}): {error_text}");
        }

        let bytes = response.bytes().await?;
        tracing::info!(
            "OpenAI TTS synthesized {} chars → {} bytes",
            text.len(),
            bytes.len()
        );

        Ok(bytes.to_vec())
    }

    async fn list_voices(&self) -> anyhow::Result<Vec<VoiceInfo>> {
        // OpenAI doesn't have a voices API, return known voices
        Ok(OPENAI_VOICES
            .iter()
            .map(|(id, name, desc)| VoiceInfo {
                id: (*id).to_string(),
                name: (*name).to_string(),
                language: "en".to_string(),
                gender: None,
                description: Some((*desc).to_string()),
            })
            .collect())
    }

    fn default_voice(&self) -> &str {
        &self.default_voice
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_defaults() {
        let tts = OpenAiTts::new("sk-test".to_string(), None, None);
        assert_eq!(tts.model, "tts-1");
        assert_eq!(tts.default_voice, "alloy");
        assert_eq!(tts.base_url, "https://api.openai.com");
    }

    #[test]
    fn creates_with_custom_model() {
        let tts = OpenAiTts::new(
            "sk-test".to_string(),
            Some("tts-1-hd".to_string()),
            Some("nova".to_string()),
        );
        assert_eq!(tts.model, "tts-1-hd");
        assert_eq!(tts.default_voice, "nova");
    }

    #[test]
    fn creates_with_custom_base_url() {
        let tts = OpenAiTts::with_base_url(
            "sk-test".to_string(),
            "https://api.custom.com/",
            None,
            None,
        );
        assert_eq!(tts.base_url, "https://api.custom.com");
    }

    #[test]
    fn format_conversion() {
        assert_eq!(OpenAiTts::format_to_response_format(AudioFormat::Mp3), "mp3");
        assert_eq!(
            OpenAiTts::format_to_response_format(AudioFormat::Opus),
            "opus"
        );
        assert_eq!(
            OpenAiTts::format_to_response_format(AudioFormat::Flac),
            "flac"
        );
        // OGG falls back to Opus
        assert_eq!(
            OpenAiTts::format_to_response_format(AudioFormat::Ogg),
            "opus"
        );
    }

    #[tokio::test]
    async fn list_voices_returns_known() {
        let tts = OpenAiTts::new("sk-test".to_string(), None, None);
        let voices = tts.list_voices().await.expect("should return voices");
        assert_eq!(voices.len(), 6);
        assert!(voices.iter().any(|v| v.id == "alloy"));
        assert!(voices.iter().any(|v| v.id == "nova"));
    }

    #[test]
    fn provider_name() {
        let tts = OpenAiTts::new("sk-test".to_string(), None, None);
        assert_eq!(tts.provider_name(), "openai");
    }

    #[tokio::test]
    async fn synthesize_fails_with_invalid_key() {
        let tts = OpenAiTts::new("invalid-key".to_string(), None, None);
        let result = tts.synthesize("Hello", None).await;
        assert!(result.is_err());
    }
}
