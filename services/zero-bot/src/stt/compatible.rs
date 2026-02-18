//! OpenAI-compatible STT implementation for providers like `UniAPI`, Groq, etc.

use super::traits::SpeechToText;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde::Deserialize;

/// Minimum audio size in bytes (1KB) - smaller files often cause parsing issues
const MIN_AUDIO_BYTES: usize = 1024;

/// `OpenAI`-compatible API implementation for speech-to-text.
/// Supports any provider that implements the `OpenAI` audio transcription API.
pub struct CompatibleStt {
    api_key: String,
    client: Client,
    model: String,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

impl CompatibleStt {
    /// Create a new OpenAI-compatible STT client.
    ///
    /// # Arguments
    /// * `api_key` - API key for the provider (can be empty for local services)
    /// * `base_url` - Base URL for the API (e.g., `https://hk.uniapi.io` or `http://localhost:8000`)
    /// * `model` - Model name (default: "whisper-1")
    pub fn new(api_key: String, base_url: String, model: Option<String>) -> Self {
        // Normalize base URL (remove trailing slash)
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                // Force HTTP/1.1 - some servers have issues with HTTP/2 and multipart forms
                .http1_only()
                .build()
                .unwrap_or_else(|_| Client::new()),
            model: model.unwrap_or_else(|| "whisper-1".to_string()),
            base_url,
        }
    }

    #[allow(dead_code)] // Keep for future use when providers support proper MIME types
    #[allow(clippy::match_same_arms)]
    fn get_mime_type(format: &str) -> &'static str {
        match format.to_lowercase().as_str() {
            "ogg" | "oga" => "audio/ogg",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "m4a" => "audio/m4a",
            "webm" => "audio/webm",
            "flac" => "audio/flac",
            _ => "audio/ogg",
        }
    }

    #[allow(clippy::match_same_arms)]
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
impl SpeechToText for CompatibleStt {
    async fn transcribe(&self, audio_bytes: &[u8], format: &str) -> anyhow::Result<String> {
        // Validate input
        if audio_bytes.is_empty() {
            anyhow::bail!("Cannot transcribe empty audio data");
        }

        // Check minimum size - very small files often cause multipart parsing issues
        if audio_bytes.len() < MIN_AUDIO_BYTES {
            anyhow::bail!(
                "Audio file too small ({} bytes, minimum {} bytes). Voice message may be too short.",
                audio_bytes.len(),
                MIN_AUDIO_BYTES
            );
        }

        // Log audio size for debugging
        tracing::debug!(
            "Transcribing {} bytes of {} audio with model {} to {}",
            audio_bytes.len(),
            format,
            self.model,
            self.base_url
        );

        let extension = Self::get_file_extension(format);
        let filename = format!("audio.{extension}");

        // Use application/octet-stream as some providers have issues with audio/* MIME types
        // in multipart forms. The file extension provides format hint.
        let part = Part::bytes(audio_bytes.to_vec())
            .file_name(filename.clone())
            .mime_str("application/octet-stream")?;

        let form = Form::new()
            .text("model", self.model.clone())
            .part("file", part);

        let url = format!("{}/v1/audio/transcriptions", self.base_url);

        tracing::debug!(
            "Sending STT request: url={}, model={}, file={}, size={}",
            url,
            self.model,
            filename,
            audio_bytes.len()
        );

        // Build request - only add Authorization header if api_key is non-empty
        // This allows local services (like faster-whisper-server) that don't require auth
        let mut request = self.client.post(&url).multipart(form);
        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("STT request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!(
                "STT API error: status={}, audio_size={}, error={}",
                status,
                audio_bytes.len(),
                error_text
            );
            anyhow::bail!("STT API error ({status}): {error_text}");
        }

        let transcription: TranscriptionResponse = response.json().await?;
        tracing::debug!(
            "STT transcription successful: {} chars",
            transcription.text.len()
        );
        Ok(transcription.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_default_model() {
        let stt = CompatibleStt::new(
            "sk-test".to_string(),
            "https://api.example.com".to_string(),
            None,
        );
        assert_eq!(stt.model, "whisper-1");
        assert_eq!(stt.base_url, "https://api.example.com");
    }

    #[test]
    fn creates_with_custom_model() {
        let stt = CompatibleStt::new(
            "sk-test".to_string(),
            "https://api.example.com/".to_string(),
            Some("qwen3-asr-flash".to_string()),
        );
        assert_eq!(stt.model, "qwen3-asr-flash");
        // Trailing slash should be removed
        assert_eq!(stt.base_url, "https://api.example.com");
    }

    #[test]
    fn mime_type_mapping() {
        assert_eq!(CompatibleStt::get_mime_type("ogg"), "audio/ogg");
        assert_eq!(CompatibleStt::get_mime_type("mp3"), "audio/mpeg");
        assert_eq!(CompatibleStt::get_mime_type("wav"), "audio/wav");
    }

    #[test]
    fn creates_with_empty_api_key() {
        // Local services (faster-whisper-server) don't require API key
        let stt = CompatibleStt::new(
            String::new(),
            "http://localhost:8000".to_string(),
            Some("base".to_string()),
        );
        assert!(stt.api_key.is_empty());
        assert_eq!(stt.model, "base");
        assert_eq!(stt.base_url, "http://localhost:8000");
    }
}
