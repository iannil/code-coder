//! Text-to-speech trait definition.

use async_trait::async_trait;

/// Voice information returned by TTS providers.
#[derive(Debug, Clone)]
pub struct VoiceInfo {
    /// Voice ID used for synthesis
    pub id: String,
    /// Human-readable voice name
    pub name: String,
    /// Voice language/locale (e.g., "en-US", "zh-CN")
    pub language: String,
    /// Voice gender (optional)
    pub gender: Option<String>,
    /// Voice description (optional)
    pub description: Option<String>,
}

/// Output audio format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioFormat {
    /// MP3 audio (most compatible)
    #[default]
    Mp3,
    /// Opus audio (smaller, good for streaming)
    Opus,
    /// AAC audio
    Aac,
    /// FLAC audio (lossless)
    Flac,
    /// WAV audio (uncompressed)
    Wav,
    /// OGG Vorbis
    Ogg,
}

impl AudioFormat {
    /// Get the file extension for this format.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Opus => "opus",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
        }
    }

    /// Get the MIME type for this format.
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/opus",
            Self::Aac => "audio/aac",
            Self::Flac => "audio/flac",
            Self::Wav => "audio/wav",
            Self::Ogg => "audio/ogg",
        }
    }
}

/// Text-to-speech synthesis options.
#[derive(Debug, Clone, Default)]
pub struct SynthesisOptions {
    /// Voice ID to use (provider-specific)
    pub voice: Option<String>,
    /// Output audio format
    pub format: AudioFormat,
    /// Speaking speed (0.25 - 4.0, default 1.0)
    pub speed: Option<f32>,
    /// Speaking pitch (0.5 - 2.0, default 1.0, not all providers support)
    pub pitch: Option<f32>,
}

/// Text-to-speech trait for converting text to audio.
///
/// Implementations should handle text input and return audio bytes
/// in the requested format.
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    /// Synthesize text to audio.
    ///
    /// # Arguments
    /// * `text` - Text to convert to speech
    /// * `options` - Synthesis options (voice, format, speed, etc.)
    ///
    /// # Returns
    /// Raw audio bytes in the requested format.
    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> anyhow::Result<Vec<u8>>;

    /// List available voices for this provider.
    ///
    /// Not all providers support listing voices, so this may return an empty list.
    async fn list_voices(&self) -> anyhow::Result<Vec<VoiceInfo>> {
        Ok(Vec::new())
    }

    /// Get the default voice ID for this provider.
    fn default_voice(&self) -> &str;

    /// Get the provider name.
    fn provider_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_format_extension() {
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Opus.extension(), "opus");
        assert_eq!(AudioFormat::Ogg.extension(), "ogg");
    }

    #[test]
    fn audio_format_mime_type() {
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
        assert_eq!(AudioFormat::Opus.mime_type(), "audio/opus");
        assert_eq!(AudioFormat::Ogg.mime_type(), "audio/ogg");
    }

    #[test]
    fn default_synthesis_options() {
        let opts = SynthesisOptions::default();
        assert!(opts.voice.is_none());
        assert_eq!(opts.format, AudioFormat::Mp3);
        assert!(opts.speed.is_none());
        assert!(opts.pitch.is_none());
    }
}
