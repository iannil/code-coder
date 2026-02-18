//! `ElevenLabs` TTS implementation.
//!
//! Uses the `ElevenLabs` API for high-quality text-to-speech synthesis.
//! Supports multiple voices and multilingual models.

use super::traits::{AudioFormat, SynthesisOptions, TextToSpeech, VoiceInfo};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

const ELEVENLABS_API_BASE: &str = "https://api.elevenlabs.io";

/// `ElevenLabs` API response for voice listing.
#[derive(Debug, Deserialize)]
struct VoicesResponse {
    voices: Vec<ElevenLabsVoice>,
}

#[derive(Debug, Deserialize)]
struct ElevenLabsVoice {
    voice_id: String,
    name: String,
    labels: Option<VoiceLabels>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VoiceLabels {
    accent: Option<String>,
    gender: Option<String>,
    age: Option<String>,
}

/// `ElevenLabs` TTS API implementation.
pub struct ElevenLabsTts {
    api_key: String,
    client: Client,
    model: String,
    default_voice: String,
}

impl ElevenLabsTts {
    /// Create a new `ElevenLabs` TTS client.
    ///
    /// # Arguments
    /// * `api_key` - `ElevenLabs` API key
    /// * `model` - Model name (default: `eleven_multilingual_v2`)
    /// * `voice` - Default voice ID (default: "21m00Tcm4TlvDq8ikWAM" - Rachel)
    pub fn new(api_key: String, model: Option<String>, voice: Option<String>) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            model: model.unwrap_or_else(|| "eleven_multilingual_v2".to_string()),
            // Rachel - one of the default ElevenLabs voices
            default_voice: voice.unwrap_or_else(|| "21m00Tcm4TlvDq8ikWAM".to_string()),
        }
    }

    fn format_to_output_format(format: AudioFormat) -> &'static str {
        match format {
            AudioFormat::Mp3 | AudioFormat::Aac | AudioFormat::Flac => "mp3_44100_128",
            AudioFormat::Opus | AudioFormat::Ogg => "opus_22050",
            AudioFormat::Wav => "pcm_44100",
        }
    }
}

#[async_trait]
impl TextToSpeech for ElevenLabsTts {
    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> anyhow::Result<Vec<u8>> {
        let opts = options.unwrap_or_default();
        let voice = opts.voice.as_deref().unwrap_or(&self.default_voice);
        let output_format = Self::format_to_output_format(opts.format);

        let url = format!(
            "{ELEVENLABS_API_BASE}/v1/text-to-speech/{voice}?output_format={output_format}"
        );

        let mut voice_settings = serde_json::json!({
            "stability": 0.5,
            "similarity_boost": 0.75
        });

        // Apply speed if specified (ElevenLabs doesn't have direct speed control,
        // but we can adjust through voice settings)
        if let Some(speed) = opts.speed {
            // ElevenLabs uses stability (lower = more variable/expressive)
            // We can approximate speed control through stability
            let stability = (1.0 / speed.clamp(0.5, 2.0)).clamp(0.3, 0.8);
            voice_settings["stability"] = serde_json::json!(stability);
        }

        let body = serde_json::json!({
            "text": text,
            "model_id": self.model,
            "voice_settings": voice_settings
        });

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs TTS API error ({status}): {error_text}");
        }

        let bytes = response.bytes().await?;
        tracing::info!(
            "ElevenLabs TTS synthesized {} chars → {} bytes",
            text.len(),
            bytes.len()
        );

        Ok(bytes.to_vec())
    }

    async fn list_voices(&self) -> anyhow::Result<Vec<VoiceInfo>> {
        let url = format!("{ELEVENLABS_API_BASE}/v1/voices");

        let response = self
            .client
            .get(&url)
            .header("xi-api-key", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs voices API error ({status}): {error_text}");
        }

        let voices_response: VoicesResponse = response.json().await?;

        Ok(voices_response
            .voices
            .into_iter()
            .map(|v| {
                let language = v
                    .labels
                    .as_ref()
                    .and_then(|l| l.accent.clone())
                    .unwrap_or_else(|| "en".to_string());
                let gender = v.labels.as_ref().and_then(|l| l.gender.clone());

                VoiceInfo {
                    id: v.voice_id,
                    name: v.name,
                    language,
                    gender,
                    description: v.description,
                }
            })
            .collect())
    }

    fn default_voice(&self) -> &str {
        &self.default_voice
    }

    fn provider_name(&self) -> &str {
        "elevenlabs"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_defaults() {
        let tts = ElevenLabsTts::new("xi-test".to_string(), None, None);
        assert_eq!(tts.model, "eleven_multilingual_v2");
        assert_eq!(tts.default_voice, "21m00Tcm4TlvDq8ikWAM");
    }

    #[test]
    fn creates_with_custom_model() {
        let tts = ElevenLabsTts::new(
            "xi-test".to_string(),
            Some("eleven_turbo_v2".to_string()),
            Some("custom-voice-id".to_string()),
        );
        assert_eq!(tts.model, "eleven_turbo_v2");
        assert_eq!(tts.default_voice, "custom-voice-id");
    }

    #[test]
    fn format_conversion() {
        assert_eq!(
            ElevenLabsTts::format_to_output_format(AudioFormat::Mp3),
            "mp3_44100_128"
        );
        assert_eq!(
            ElevenLabsTts::format_to_output_format(AudioFormat::Opus),
            "opus_22050"
        );
        assert_eq!(
            ElevenLabsTts::format_to_output_format(AudioFormat::Wav),
            "pcm_44100"
        );
    }

    #[test]
    fn provider_name() {
        let tts = ElevenLabsTts::new("xi-test".to_string(), None, None);
        assert_eq!(tts.provider_name(), "elevenlabs");
    }

    #[tokio::test]
    async fn synthesize_fails_with_invalid_key() {
        let tts = ElevenLabsTts::new("invalid-key".to_string(), None, None);
        let result = tts.synthesize("Hello", None).await;
        assert!(result.is_err());
    }
}
