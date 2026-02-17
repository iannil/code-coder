use super::traits::SpeechToText;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;

/// `OpenAI` Whisper API implementation for speech-to-text.
pub struct OpenAiStt {
    api_key: String,
    client: Client,
    model: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

impl OpenAiStt {
    /// Create a new `OpenAI` STT client.
    ///
    /// # Arguments
    /// * `api_key` - `OpenAI` API key
    /// * `model` - Model name (default: "whisper-1")
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            model: model.unwrap_or_else(|| "whisper-1".to_string()),
        }
    }

    #[allow(clippy::match_same_arms)] // Keep explicit patterns for documentation
    fn get_mime_type(format: &str) -> &'static str {
        match format.to_lowercase().as_str() {
            "ogg" | "oga" => "audio/ogg",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "m4a" => "audio/m4a",
            "webm" => "audio/webm",
            "flac" => "audio/flac",
            _ => "audio/ogg", // Default for Telegram voice messages
        }
    }

    #[allow(clippy::match_same_arms)] // Keep explicit patterns for documentation
    fn get_file_extension(format: &str) -> &'static str {
        match format.to_lowercase().as_str() {
            "ogg" | "oga" => "ogg",
            "mp3" => "mp3",
            "wav" => "wav",
            "m4a" => "m4a",
            "webm" => "webm",
            "flac" => "flac",
            _ => "ogg",
        }
    }
}

#[async_trait]
impl SpeechToText for OpenAiStt {
    async fn transcribe(&self, audio_bytes: &[u8], format: &str) -> anyhow::Result<String> {
        let mime_type = Self::get_mime_type(format);
        let extension = Self::get_file_extension(format);
        let filename = format!("audio.{extension}");

        let part = Part::bytes(audio_bytes.to_vec())
            .file_name(filename)
            .mime_str(mime_type)?;

        let form = Form::new()
            .text("model", self.model.clone())
            .part("file", part);

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI Whisper API error ({status}): {error_text}");
        }

        let transcription: TranscriptionResponse = response.json().await?;
        Ok(transcription.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_default_model() {
        let stt = OpenAiStt::new("sk-test".to_string(), None);
        assert_eq!(stt.model, "whisper-1");
        assert_eq!(stt.api_key, "sk-test");
    }

    #[test]
    fn creates_with_custom_model() {
        let stt = OpenAiStt::new("sk-test".to_string(), Some("whisper-large-v3".to_string()));
        assert_eq!(stt.model, "whisper-large-v3");
    }

    #[test]
    fn mime_type_ogg() {
        assert_eq!(OpenAiStt::get_mime_type("ogg"), "audio/ogg");
        assert_eq!(OpenAiStt::get_mime_type("OGG"), "audio/ogg");
        assert_eq!(OpenAiStt::get_mime_type("oga"), "audio/ogg");
    }

    #[test]
    fn mime_type_mp3() {
        assert_eq!(OpenAiStt::get_mime_type("mp3"), "audio/mpeg");
        assert_eq!(OpenAiStt::get_mime_type("MP3"), "audio/mpeg");
    }

    #[test]
    fn mime_type_wav() {
        assert_eq!(OpenAiStt::get_mime_type("wav"), "audio/wav");
    }

    #[test]
    fn mime_type_m4a() {
        assert_eq!(OpenAiStt::get_mime_type("m4a"), "audio/m4a");
    }

    #[test]
    fn mime_type_webm() {
        assert_eq!(OpenAiStt::get_mime_type("webm"), "audio/webm");
    }

    #[test]
    fn mime_type_flac() {
        assert_eq!(OpenAiStt::get_mime_type("flac"), "audio/flac");
    }

    #[test]
    fn mime_type_unknown_defaults_to_ogg() {
        assert_eq!(OpenAiStt::get_mime_type("xyz"), "audio/ogg");
        assert_eq!(OpenAiStt::get_mime_type(""), "audio/ogg");
    }

    #[test]
    fn file_extension_mapping() {
        assert_eq!(OpenAiStt::get_file_extension("ogg"), "ogg");
        assert_eq!(OpenAiStt::get_file_extension("mp3"), "mp3");
        assert_eq!(OpenAiStt::get_file_extension("wav"), "wav");
        assert_eq!(OpenAiStt::get_file_extension("unknown"), "ogg");
    }

    #[tokio::test]
    async fn transcribe_fails_with_invalid_key() {
        let stt = OpenAiStt::new("invalid-key".to_string(), None);
        let audio = b"fake audio data";
        let result = stt.transcribe(audio, "ogg").await;
        assert!(result.is_err());
    }
}
