//! Audio utilities for file I/O and format conversion
//! 
//! Provides functionality for saving audio data to various formats.

/// Audio file writer for saving generated audio
pub struct AudioWriter;

impl AudioWriter {
    /// Write audio samples to WAV file (simplified implementation)
    pub fn write_wav_file(
        filename: &str,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::{Write, BufWriter};
        
        let mut file = BufWriter::new(File::create(filename)?);
        
        // WAV header
        let num_samples = samples.len() as u32;
        let byte_rate = sample_rate * 2; // 16-bit mono
        let data_size = num_samples * 2;
        let file_size = 36 + data_size;
        
        // RIFF header
        file.write_all(b"RIFF")?;
        file.write_all(&file_size.to_le_bytes())?;
        file.write_all(b"WAVE")?;
        
        // Format chunk
        file.write_all(b"fmt ")?;
        file.write_all(&16u32.to_le_bytes())?; // Chunk size
        file.write_all(&1u16.to_le_bytes())?;  // Audio format (PCM)
        file.write_all(&1u16.to_le_bytes())?;  // Num channels
        file.write_all(&sample_rate.to_le_bytes())?;
        file.write_all(&byte_rate.to_le_bytes())?;
        file.write_all(&2u16.to_le_bytes())?;  // Block align
        file.write_all(&16u16.to_le_bytes())?; // Bits per sample
        
        // Data chunk
        file.write_all(b"data")?;
        file.write_all(&data_size.to_le_bytes())?;
        
        // Audio data (convert f32 to i16)
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
            file.write_all(&sample_i16.to_le_bytes())?;
        }
        
        file.flush()?;
        Ok(())
    }
    
    /// Get audio format information
    pub fn get_format_info(filename: &str) -> Option<AudioFormatInfo> {
        // Simple format detection based on extension
        let extension = std::path::Path::new(filename)
            .extension()?
            .to_str()?
            .to_lowercase();
            
        match extension.as_str() {
            "wav" => Some(AudioFormatInfo {
                format: AudioFormat::Wav,
                typical_sample_rate: 44100,
                bits_per_sample: 16,
                channels: 1,
            }),
            "mp3" => Some(AudioFormatInfo {
                format: AudioFormat::Mp3,
                typical_sample_rate: 44100,
                bits_per_sample: 16,
                channels: 2,
            }),
            "flac" => Some(AudioFormatInfo {
                format: AudioFormat::Flac,
                typical_sample_rate: 48000,
                bits_per_sample: 24,
                channels: 2,
            }),
            _ => None,
        }
    }
}

/// Audio format enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Ogg,
}

/// Audio format information
#[derive(Debug, Clone)]
pub struct AudioFormatInfo {
    pub format: AudioFormat,
    pub typical_sample_rate: u32,
    pub bits_per_sample: u8,
    pub channels: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        assert_eq!(
            AudioWriter::get_format_info("test.wav").unwrap().format,
            AudioFormat::Wav
        );
        assert_eq!(
            AudioWriter::get_format_info("test.mp3").unwrap().format,
            AudioFormat::Mp3
        );
        assert!(AudioWriter::get_format_info("test.unknown").is_none());
    }
}