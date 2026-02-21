//! Speech-to-text trait definition.

use async_trait::async_trait;

/// Speech-to-text trait for transcribing audio to text.
///
/// Implementations should handle audio data in various formats (ogg, mp3, wav, etc.)
/// and return the transcribed text.
#[async_trait]
pub trait SpeechToText: Send + Sync {
    /// Transcribe audio bytes to text.
    ///
    /// # Arguments
    /// * `audio_bytes` - Raw audio data
    /// * `format` - Audio format hint (e.g., "ogg", "mp3", "wav")
    ///
    /// # Returns
    /// The transcribed text, or an error if transcription fails.
    async fn transcribe(&self, audio_bytes: &[u8], format: &str) -> anyhow::Result<String>;

    /// Get the provider name.
    fn provider_name(&self) -> &str;
}
