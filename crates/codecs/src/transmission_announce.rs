//! Complete transmission announcements combining CW and voice
//! 
//! Provides functionality to generate complete audio announcements
//! including optional pink noise burst, CW preambles and voice identification.

use crate::cw::{CwGenerator, CwConfig};
use crate::voice_announce::{VoiceAnnouncer, PinkNoiseGenerator};
use std::path::Path;

/// Configuration for transmission announcements
#[derive(Debug, Clone)]
pub struct AnnouncementConfig {
    /// Enable pink noise burst before transmission for squelch triggering
    pub enable_pink_noise: bool,
    /// Duration of pink noise burst in seconds
    pub pink_noise_duration: f32,
    /// Amplitude of pink noise burst (0.0 to 1.0)
    pub pink_noise_amplitude: f32,
    /// Enable CW announcement
    pub enable_cw: bool,
    /// Enable voice announcement
    pub enable_voice: bool,
    /// Delay between CW and voice announcements in seconds
    pub announcement_delay: f32,
}

impl Default for AnnouncementConfig {
    fn default() -> Self {
        Self {
            enable_pink_noise: true,
            pink_noise_duration: 0.5, // Half second
            pink_noise_amplitude: 0.1, // 10% amplitude
            enable_cw: true,
            enable_voice: true,
            announcement_delay: 0.5,
        }
    }
}

/// Complete transmission announcement generator
pub struct TransmissionAnnouncer {
    voice_announcer: VoiceAnnouncer,
    pink_noise_generator: PinkNoiseGenerator,
    config: AnnouncementConfig,
}

impl TransmissionAnnouncer {
    /// Create a new transmission announcer
    pub fn new(sample_rate: f64) -> Self {
        Self {
            voice_announcer: VoiceAnnouncer::new(sample_rate),
            pink_noise_generator: PinkNoiseGenerator::new(),
            config: AnnouncementConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(sample_rate: f64, config: AnnouncementConfig) -> Self {
        Self {
            voice_announcer: VoiceAnnouncer::new(sample_rate),
            pink_noise_generator: PinkNoiseGenerator::new(),
            config,
        }
    }
    
    /// Set announcement configuration
    pub fn set_config(&mut self, config: AnnouncementConfig) {
        self.config = config;
    }
    
    /// Get current configuration
    pub fn config(&self) -> &AnnouncementConfig {
        &self.config
    }
    
    /// Generate complete audio announcement with optional pink noise, CW, and voice from file
    pub fn generate_complete_announcement<P: AsRef<Path>>(
        &mut self,
        callsign: &str,
        mode: &str,
        frequency: Option<f64>,
        cw_config: &CwConfig,
        voice_file_path: Option<P>,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut combined = Vec::new();
        
        // 1. Optional pink noise burst for squelch triggering
        if self.config.enable_pink_noise {
            let noise_samples = (self.config.pink_noise_duration * cw_config.sample_rate as f32) as usize;
            let mut noise = self.pink_noise_generator.generate_samples(
                noise_samples, 
                self.config.pink_noise_amplitude
            );
            combined.append(&mut noise);
            
            // Brief silence after pink noise
            let silence_samples = (0.1 * cw_config.sample_rate) as usize; // 100ms
            combined.extend(vec![0.0; silence_samples]);
        }
        
        // 2. Optional CW preamble
        if self.config.enable_cw {
            let cw_generator = CwGenerator::new(cw_config.clone());
            let mut cw_audio = cw_generator.generate_preamble(callsign, mode, frequency);
            combined.append(&mut cw_audio);
            
            // Delay between CW and voice if both are enabled
            if self.config.enable_voice && voice_file_path.is_some() {
                let delay_samples = (self.config.announcement_delay * cw_config.sample_rate as f32) as usize;
                combined.extend(vec![0.0; delay_samples]);
            }
        }
        
        // 3. Optional voice announcement from file
        if self.config.enable_voice {
            if let Some(voice_path) = voice_file_path {
                match self.voice_announcer.load_announcement_from_file(voice_path) {
                    Ok(mut voice_audio) => {
                        combined.append(&mut voice_audio);
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not load voice announcement: {}", e);
                        // Continue without voice announcement
                    }
                }
            }
        }
        
        // Final silence before data transmission
        let final_silence = (0.2 * cw_config.sample_rate) as usize; // 200ms
        combined.extend(vec![0.0; final_silence]);
        
        Ok(combined)
    }
    
    /// Generate CW-only announcement with optional pink noise
    pub fn generate_cw_announcement(
        &mut self,
        callsign: &str,
        mode: &str,
        frequency: Option<f64>,
        cw_config: &CwConfig,
    ) -> Vec<f32> {
        let mut combined = Vec::new();
        
        // Optional pink noise
        if self.config.enable_pink_noise {
            let noise_samples = (self.config.pink_noise_duration * cw_config.sample_rate as f32) as usize;
            let mut noise = self.pink_noise_generator.generate_samples(
                noise_samples, 
                self.config.pink_noise_amplitude
            );
            combined.append(&mut noise);
            
            let silence_samples = (0.1 * cw_config.sample_rate) as usize;
            combined.extend(vec![0.0; silence_samples]);
        }
        
        // CW preamble
        let cw_generator = CwGenerator::new(cw_config.clone());
        let mut cw_audio = cw_generator.generate_preamble(callsign, mode, frequency);
        combined.append(&mut cw_audio);
        
        combined
    }
    
    /// Generate voice-only announcement from file with optional pink noise
    pub fn generate_voice_announcement<P: AsRef<Path>>(
        &mut self,
        voice_file_path: P,
        sample_rate: f64,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut combined = Vec::new();
        
        // Optional pink noise
        if self.config.enable_pink_noise {
            let noise_samples = (self.config.pink_noise_duration * sample_rate as f32) as usize;
            let mut noise = self.pink_noise_generator.generate_samples(
                noise_samples, 
                self.config.pink_noise_amplitude
            );
            combined.append(&mut noise);
            
            let silence_samples = (0.1 * sample_rate) as usize;
            combined.extend(vec![0.0; silence_samples]);
        }
        
        // Voice announcement from file
        let mut voice_audio = self.voice_announcer.load_announcement_from_file(voice_file_path)?;
        combined.append(&mut voice_audio);
        
        Ok(combined)
    }
    
    /// Generate only pink noise burst (for testing squelch triggering)
    pub fn generate_pink_noise_burst(&mut self, sample_rate: f64) -> Vec<f32> {
        if !self.config.enable_pink_noise {
            return Vec::new();
        }
        
        let noise_samples = (self.config.pink_noise_duration * sample_rate as f32) as usize;
        self.pink_noise_generator.generate_samples(
            noise_samples, 
            self.config.pink_noise_amplitude
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_announcement_config_default() {
        let config = AnnouncementConfig::default();
        assert!(config.enable_pink_noise);
        assert_eq!(config.pink_noise_duration, 0.5);
        assert_eq!(config.pink_noise_amplitude, 0.1);
        assert!(config.enable_cw);
        assert!(config.enable_voice);
    }

    #[test]
    fn test_transmission_announcer_creation() {
        let announcer = TransmissionAnnouncer::new(8000.0);
        assert!(announcer.config.enable_pink_noise);
    }

    #[test]
    fn test_cw_only_announcement() {
        let mut announcer = TransmissionAnnouncer::new(8000.0);
        let cw_config = CwConfig::new(20, 600.0, 8000.0);
        
        let cw_only = announcer.generate_cw_announcement(
            "N0CALL",
            "PSK31",
            Some(14070000.0),
            &cw_config,
        );
        
        assert!(!cw_only.is_empty());
        
        // Should be longer than just the pink noise if enabled
        let expected_min_length = if announcer.config.enable_pink_noise {
            (announcer.config.pink_noise_duration * cw_config.sample_rate as f32) as usize
        } else {
            0
        };
        assert!(cw_only.len() > expected_min_length);
    }

    #[test]
    fn test_pink_noise_burst() {
        let mut announcer = TransmissionAnnouncer::new(8000.0);
        let burst = announcer.generate_pink_noise_burst(8000.0);
        
        // Should generate noise if enabled
        if announcer.config.enable_pink_noise {
            assert!(!burst.is_empty());
            let expected_length = (announcer.config.pink_noise_duration * 8000.0) as usize;
            assert_eq!(burst.len(), expected_length);
        }
    }

    #[test]
    fn test_config_modification() {
        let mut announcer = TransmissionAnnouncer::new(8000.0);
        
        let mut custom_config = AnnouncementConfig::default();
        custom_config.enable_pink_noise = false;
        custom_config.pink_noise_amplitude = 0.05;
        
        announcer.set_config(custom_config.clone());
        assert_eq!(announcer.config().enable_pink_noise, false);
        assert_eq!(announcer.config().pink_noise_amplitude, 0.05);
    }

    #[test]
    fn test_voice_announcement_with_missing_file() {
        let mut announcer = TransmissionAnnouncer::new(8000.0);
        let result = announcer.generate_voice_announcement("nonexistent.wav", 8000.0);
        
        // Should return an error for missing file
        assert!(result.is_err());
    }

    #[test]
    fn test_disabled_pink_noise() {
        let mut config = AnnouncementConfig::default();
        config.enable_pink_noise = false;
        
        let mut announcer = TransmissionAnnouncer::with_config(8000.0, config);
        let burst = announcer.generate_pink_noise_burst(8000.0);
        
        // Should return empty vector when disabled
        assert!(burst.is_empty());
    }

    #[test]
    fn test_complete_announcement_error_handling() {
        let mut announcer = TransmissionAnnouncer::new(8000.0);
        let cw_config = CwConfig::new(20, 600.0, 8000.0);
        
        // Test with missing voice file - should continue without voice
        let result = announcer.generate_complete_announcement(
            "N0CALL",
            "PSK31",
            Some(14070000.0),
            &cw_config,
            Some("nonexistent.wav"),
        );
        
        // Should succeed even with missing voice file
        assert!(result.is_ok());
        let complete = result.unwrap();
        assert!(!complete.is_empty());
    }
}