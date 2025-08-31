//! Voice announcement playback for amateur radio
//! 
//! Provides functionality to play pre-recorded voice announcements
//! for station identification and mode announcements.

use std::path::Path;

/// Voice announcement player for pre-recorded audio files
pub struct VoiceAnnouncer {
    sample_rate: f64,
}

impl VoiceAnnouncer {
    pub fn new(sample_rate: f64) -> Self {
        Self { sample_rate }
    }
    
    /// Load and play pre-recorded voice announcement from WAV file
    pub fn load_announcement_from_file<P: AsRef<Path>>(
        &self, 
        audio_file_path: P
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        // Simple WAV file reader implementation
        // In a real implementation, you might want to use a library like `hound`
        self.read_wav_file(audio_file_path.as_ref())
    }
    
    /// Load announcement from raw PCM data
    pub fn load_announcement_from_pcm(&self, pcm_data: &[f32]) -> Vec<f32> {
        pcm_data.to_vec()
    }
    
    /// Resample audio if needed (simple linear interpolation)
    pub fn resample_if_needed(&self, audio_data: &[f32], source_sample_rate: f64) -> Vec<f32> {
        if (source_sample_rate - self.sample_rate).abs() < 1.0 {
            // Sample rates are close enough, no resampling needed
            return audio_data.to_vec();
        }
        
        let ratio = self.sample_rate / source_sample_rate;
        let new_length = (audio_data.len() as f64 * ratio) as usize;
        let mut resampled = Vec::with_capacity(new_length);
        
        for i in 0..new_length {
            let source_index = i as f64 / ratio;
            let index_floor = source_index.floor() as usize;
            let index_ceil = (index_floor + 1).min(audio_data.len() - 1);
            let fraction = source_index - index_floor as f64;
            
            if index_floor < audio_data.len() {
                let sample = if index_floor == index_ceil {
                    audio_data[index_floor]
                } else {
                    // Linear interpolation
                    audio_data[index_floor] * (1.0 - fraction) as f32 + 
                    audio_data[index_ceil] * fraction as f32
                };
                resampled.push(sample);
            }
        }
        
        resampled
    }
    
    /// Simple WAV file reader (basic implementation)
    fn read_wav_file(&self, path: &Path) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::{Read, BufReader};
        
        let mut file = BufReader::new(File::open(path)?);
        let mut header = [0u8; 44]; // Standard WAV header size
        file.read_exact(&mut header)?;
        
        // Verify RIFF/WAVE header
        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err("Invalid WAV file format".into());
        }
        
        // Extract format information
        let num_channels = u16::from_le_bytes([header[22], header[23]]);
        let sample_rate = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
        let bits_per_sample = u16::from_le_bytes([header[34], header[35]]);
        
        // Read audio data
        let mut raw_data = Vec::new();
        file.read_to_end(&mut raw_data)?;
        
        // Convert to f32 samples
        let mut samples = Vec::new();
        match bits_per_sample {
            16 => {
                for chunk in raw_data.chunks(2) {
                    if chunk.len() == 2 {
                        let sample_i16 = i16::from_le_bytes([chunk[0], chunk[1]]);
                        let sample_f32 = sample_i16 as f32 / 32768.0;
                        samples.push(sample_f32);
                    }
                }
            }
            8 => {
                for &byte in &raw_data {
                    let sample_f32 = (byte as i8 as f32) / 128.0;
                    samples.push(sample_f32);
                }
            }
            _ => return Err("Unsupported bit depth".into()),
        }
        
        // Convert stereo to mono if needed
        if num_channels == 2 {
            let mono_samples: Vec<f32> = samples
                .chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).unwrap_or(&0.0)) / 2.0)
                .collect();
            samples = mono_samples;
        }
        
        // Resample if needed
        Ok(self.resample_if_needed(&samples, sample_rate as f64))
    }
}

/// Pink noise generator for squelch triggering
pub struct PinkNoiseGenerator {
    /// Previous values for pink noise filtering
    b0: f32,
    b1: f32,
    b2: f32,
    b3: f32,
    b4: f32,
    b5: f32,
    b6: f32,
}

impl PinkNoiseGenerator {
    pub fn new() -> Self {
        Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            b3: 0.0,
            b4: 0.0,
            b5: 0.0,
            b6: 0.0,
        }
    }
    
    /// Generate pink noise samples using Paul Kellett's algorithm
    pub fn generate_samples(&mut self, num_samples: usize, amplitude: f32) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples);
        
        // Simple PRNG state
        let mut state: u32 = 0x12345678;
        
        for _ in 0..num_samples {
            // Simple linear congruential generator for pseudo-random numbers
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            let white = ((state as f32) / (u32::MAX as f32) * 2.0 - 1.0) * 0.2;
            
            // Apply pink noise filter (Paul Kellett's method)
            self.b0 = 0.99886 * self.b0 + white * 0.0555179;
            self.b1 = 0.99332 * self.b1 + white * 0.0750759;
            self.b2 = 0.96900 * self.b2 + white * 0.1538520;
            self.b3 = 0.86650 * self.b3 + white * 0.3104856;
            self.b4 = 0.55000 * self.b4 + white * 0.5329522;
            self.b5 = -0.7616 * self.b5 - white * 0.0168980;
            
            let pink = self.b0 + self.b1 + self.b2 + self.b3 + self.b4 + self.b5 + self.b6 + white * 0.5362;
            self.b6 = white * 0.115926;
            
            samples.push(pink * amplitude);
        }
        
        samples
    }
}

impl Default for PinkNoiseGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_announcer_creation() {
        let announcer = VoiceAnnouncer::new(8000.0);
        assert_eq!(announcer.sample_rate, 8000.0);
    }

    #[test]
    fn test_pcm_loading() {
        let announcer = VoiceAnnouncer::new(8000.0);
        let test_data = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let loaded = announcer.load_announcement_from_pcm(&test_data);
        assert_eq!(loaded, test_data);
    }

    #[test]
    fn test_resampling() {
        let announcer = VoiceAnnouncer::new(8000.0);
        let test_data = vec![0.0, 1.0, 0.0, -1.0]; // 4 samples
        
        // Test upsampling (4kHz to 8kHz should double the length)
        let resampled = announcer.resample_if_needed(&test_data, 4000.0);
        assert!(resampled.len() > test_data.len());
        
        // Test no resampling needed
        let no_resample = announcer.resample_if_needed(&test_data, 8000.0);
        assert_eq!(no_resample.len(), test_data.len());
    }

    #[test]
    fn test_pink_noise_generation() {
        let mut generator = PinkNoiseGenerator::new();
        let samples = generator.generate_samples(1000, 0.1);
        
        assert_eq!(samples.len(), 1000);
        
        // Check that we have some variation (not all zeros)
        let has_variation = samples.iter().any(|&s| s.abs() > 0.01);
        assert!(has_variation);
        
        // Check amplitude is reasonable
        let max_amplitude = samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(max_amplitude < 1.0); // Should be within reasonable bounds
    }

    #[test]
    fn test_wav_file_loading_error_handling() {
        let announcer = VoiceAnnouncer::new(8000.0);
        
        // Test with non-existent file
        let result = announcer.load_announcement_from_file("nonexistent.wav");
        assert!(result.is_err());
    }
}