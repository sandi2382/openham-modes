//! CW (Morse Code) generation for amateur radio
//! 
//! Generates CW preambles and morse code audio for transmissions.

use std::collections::HashMap;
use std::f64::consts::PI;

/// Morse code timing configuration
#[derive(Debug, Clone)]
pub struct CwConfig {
    /// Words per minute (WPM)
    pub wpm: u32,
    
    /// CW tone frequency in Hz
    pub tone_frequency: f64,
    
    /// Sample rate for audio generation
    pub sample_rate: f64,
    
    /// Rise/fall time for CW shaping (milliseconds)
    pub rise_fall_time_ms: f64,
    
    /// Character spacing (units of dot length)
    pub character_spacing: f64,
    
    /// Word spacing (units of dot length)
    pub word_spacing: f64,
}

impl CwConfig {
    /// Create standard CW configuration
    pub fn new(wpm: u32, tone_frequency: f64, sample_rate: f64) -> Self {
        Self {
            wpm,
            tone_frequency,
            sample_rate,
            rise_fall_time_ms: 5.0, // 5ms rise/fall time
            character_spacing: 3.0,  // 3 dot lengths between characters
            word_spacing: 7.0,       // 7 dot lengths between words
        }
    }
    
    /// Calculate dot length in seconds
    pub fn dot_length_seconds(&self) -> f64 {
        // Standard formula: dot length = 1.2 / WPM
        1.2 / self.wpm as f64
    }
    
    /// Calculate dash length in seconds
    pub fn dash_length_seconds(&self) -> f64 {
        3.0 * self.dot_length_seconds()
    }
    
    /// Calculate inter-element spacing
    pub fn element_spacing_seconds(&self) -> f64 {
        self.dot_length_seconds()
    }
    
    /// Calculate character spacing
    pub fn character_spacing_seconds(&self) -> f64 {
        self.character_spacing * self.dot_length_seconds()
    }
    
    /// Calculate word spacing
    pub fn word_spacing_seconds(&self) -> f64 {
        self.word_spacing * self.dot_length_seconds()
    }
}

/// Morse code element
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MorseElement {
    Dot,
    Dash,
    ElementSpace,
    CharacterSpace,
    WordSpace,
}

/// Morse code generator
pub struct CwGenerator {
    config: CwConfig,
    morse_table: HashMap<char, Vec<MorseElement>>,
}

impl CwGenerator {
    /// Create a new CW generator
    pub fn new(config: CwConfig) -> Self {
        let mut generator = Self {
            config,
            morse_table: HashMap::new(),
        };
        generator.init_morse_table();
        generator
    }
    
    /// Initialize the Morse code lookup table
    fn init_morse_table(&mut self) {
        use MorseElement::*;
        
        // Letters
        self.morse_table.insert('A', vec![Dot, Dash]);
        self.morse_table.insert('B', vec![Dash, Dot, Dot, Dot]);
        self.morse_table.insert('C', vec![Dash, Dot, Dash, Dot]);
        self.morse_table.insert('D', vec![Dash, Dot, Dot]);
        self.morse_table.insert('E', vec![Dot]);
        self.morse_table.insert('F', vec![Dot, Dot, Dash, Dot]);
        self.morse_table.insert('G', vec![Dash, Dash, Dot]);
        self.morse_table.insert('H', vec![Dot, Dot, Dot, Dot]);
        self.morse_table.insert('I', vec![Dot, Dot]);
        self.morse_table.insert('J', vec![Dot, Dash, Dash, Dash]);
        self.morse_table.insert('K', vec![Dash, Dot, Dash]);
        self.morse_table.insert('L', vec![Dot, Dash, Dot, Dot]);
        self.morse_table.insert('M', vec![Dash, Dash]);
        self.morse_table.insert('N', vec![Dash, Dot]);
        self.morse_table.insert('O', vec![Dash, Dash, Dash]);
        self.morse_table.insert('P', vec![Dot, Dash, Dash, Dot]);
        self.morse_table.insert('Q', vec![Dash, Dash, Dot, Dash]);
        self.morse_table.insert('R', vec![Dot, Dash, Dot]);
        self.morse_table.insert('S', vec![Dot, Dot, Dot]);
        self.morse_table.insert('T', vec![Dash]);
        self.morse_table.insert('U', vec![Dot, Dot, Dash]);
        self.morse_table.insert('V', vec![Dot, Dot, Dot, Dash]);
        self.morse_table.insert('W', vec![Dot, Dash, Dash]);
        self.morse_table.insert('X', vec![Dash, Dot, Dot, Dash]);
        self.morse_table.insert('Y', vec![Dash, Dot, Dash, Dash]);
        self.morse_table.insert('Z', vec![Dash, Dash, Dot, Dot]);
        
        // Numbers
        self.morse_table.insert('0', vec![Dash, Dash, Dash, Dash, Dash]);
        self.morse_table.insert('1', vec![Dot, Dash, Dash, Dash, Dash]);
        self.morse_table.insert('2', vec![Dot, Dot, Dash, Dash, Dash]);
        self.morse_table.insert('3', vec![Dot, Dot, Dot, Dash, Dash]);
        self.morse_table.insert('4', vec![Dot, Dot, Dot, Dot, Dash]);
        self.morse_table.insert('5', vec![Dot, Dot, Dot, Dot, Dot]);
        self.morse_table.insert('6', vec![Dash, Dot, Dot, Dot, Dot]);
        self.morse_table.insert('7', vec![Dash, Dash, Dot, Dot, Dot]);
        self.morse_table.insert('8', vec![Dash, Dash, Dash, Dot, Dot]);
        self.morse_table.insert('9', vec![Dash, Dash, Dash, Dash, Dot]);
        
        // Common punctuation
        self.morse_table.insert('/', vec![Dash, Dot, Dot, Dash, Dot]);
        self.morse_table.insert('?', vec![Dot, Dot, Dash, Dash, Dot, Dot]);
        self.morse_table.insert('.', vec![Dot, Dash, Dot, Dash, Dot, Dash]);
        self.morse_table.insert(',', vec![Dash, Dash, Dot, Dot, Dash, Dash]);
        self.morse_table.insert('-', vec![Dash, Dot, Dot, Dot, Dot, Dash]);
        self.morse_table.insert('=', vec![Dash, Dot, Dot, Dot, Dash]);
        
        // Prosigns
        self.morse_table.insert('@', vec![Dot, Dash, Dash, Dot, Dash, Dot]); // AC (message begins)
        self.morse_table.insert('+', vec![Dot, Dash, Dot, Dash, Dot]); // AR (message ends)
        self.morse_table.insert('&', vec![Dot, Dot, Dot, Dash, Dot]); // AS (wait)
        self.morse_table.insert('*', vec![Dash, Dot, Dot, Dash]); // BT (break)
        self.morse_table.insert('%', vec![Dot, Dot, Dot, Dot, Dot, Dot, Dot, Dot]); // Error (8 dots)
        self.morse_table.insert('^', vec![Dash, Dot, Dash, Dot, Dash]); // KA (attention)
        self.morse_table.insert('~', vec![Dash, Dot, Dash, Dash, Dot]); // KN (go ahead specific station)
        self.morse_table.insert('>', vec![Dot, Dash, Dot, Dot, Dash]); // SK (end of contact)
        self.morse_table.insert('<', vec![Dot, Dot, Dot, Dash, Dot, Dash]); // SN (understood)
    }
    
    /// Convert text to morse elements
    pub fn text_to_morse(&self, text: &str) -> Vec<MorseElement> {
        let mut elements = Vec::new();
        let text = text.to_uppercase();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == ' ' {
                elements.push(MorseElement::WordSpace);
            } else if let Some(morse_chars) = self.morse_table.get(&ch) {
                // Add character elements
                for (i, element) in morse_chars.iter().enumerate() {
                    elements.push(*element);
                    // Add element spacing between dots/dashes (except after last element)
                    if i < morse_chars.len() - 1 {
                        elements.push(MorseElement::ElementSpace);
                    }
                }
                
                // Add character spacing (except after last character)
                if chars.peek().is_some() && chars.peek() != Some(&' ') {
                    elements.push(MorseElement::CharacterSpace);
                }
            }
            // Unknown characters are skipped
        }
        
        elements
    }
    
    /// Generate CW audio samples from morse elements
    pub fn generate_audio(&self, elements: &[MorseElement]) -> Vec<f32> {
        let mut samples = Vec::new();
        let mut phase = 0.0;
        
        for element in elements {
            let (duration, is_tone) = match element {
                MorseElement::Dot => (self.config.dot_length_seconds(), true),
                MorseElement::Dash => (self.config.dash_length_seconds(), true),
                MorseElement::ElementSpace => (self.config.element_spacing_seconds(), false),
                MorseElement::CharacterSpace => (self.config.character_spacing_seconds(), false),
                MorseElement::WordSpace => (self.config.word_spacing_seconds(), false),
            };
            
            let num_samples = (duration * self.config.sample_rate) as usize;
            let rise_fall_samples = (self.config.rise_fall_time_ms * 0.001 * self.config.sample_rate) as usize;
            
            for i in 0..num_samples {
                let mut amplitude = if is_tone { 1.0 } else { 0.0 };
                
                // Apply rise/fall shaping for tones
                if is_tone && rise_fall_samples > 0 {
                    if i < rise_fall_samples {
                        // Rise time
                        amplitude *= i as f64 / rise_fall_samples as f64;
                    } else if i >= num_samples - rise_fall_samples {
                        // Fall time
                        let fall_progress = (num_samples - 1 - i) as f64 / rise_fall_samples as f64;
                        amplitude *= fall_progress;
                    }
                }
                
                let sample = if is_tone {
                    amplitude * (2.0 * PI * self.config.tone_frequency * phase / self.config.sample_rate).sin()
                } else {
                    0.0
                };
                
                samples.push(sample as f32);
                phase += 1.0;
                
                // Prevent phase accumulation overflow
                if phase >= self.config.sample_rate {
                    phase -= self.config.sample_rate;
                }
            }
        }
        
        samples
    }
    
    /// Generate CW audio for text
    pub fn generate_cw_audio(&self, text: &str) -> Vec<f32> {
        let elements = self.text_to_morse(text);
        self.generate_audio(&elements)
    }
    
    /// Generate standard amateur radio preamble
    pub fn generate_preamble(&self, callsign: &str, mode: &str, frequency: Option<f64>) -> Vec<f32> {
        let mut preamble_text = String::new();
        
        // Start with attention signal
        preamble_text.push_str("^^ ");  // KA KA (attention)
        
        // Add callsign
        preamble_text.push_str(&format!("DE {} ", callsign));
        
        // Add mode information if provided
        if !mode.is_empty() {
            preamble_text.push_str(&format!("{} ", mode));
        }
        
        // Add frequency if provided
        if let Some(freq) = frequency {
            if freq >= 1000000.0 {
                preamble_text.push_str(&format!("{:.3} MHZ ", freq / 1000000.0));
            } else if freq >= 1000.0 {
                preamble_text.push_str(&format!("{:.1} KHZ ", freq / 1000.0));
            } else {
                preamble_text.push_str(&format!("{:.0} HZ ", freq));
            }
        }
        
        // End with message begins signal
        preamble_text.push_str("@ +");  // AC AR
        
        self.generate_cw_audio(&preamble_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cw_config() {
        let config = CwConfig::new(20, 600.0, 48000.0);
        assert_eq!(config.wpm, 20);
        assert_eq!(config.tone_frequency, 600.0);
        
        // Test timing calculations
        let dot_length = config.dot_length_seconds();
        assert!((dot_length - 0.06).abs() < 0.001); // 20 WPM = 60ms dots
    }

    #[test]
    fn test_morse_conversion() {
        let config = CwConfig::new(20, 600.0, 48000.0);
        let generator = CwGenerator::new(config);
        
        let elements = generator.text_to_morse("SOS");
        
        // SOS = ... --- ...
        // Should have dots, dashes, and spacing
        assert!(!elements.is_empty());
        assert!(elements.contains(&MorseElement::Dot));
        assert!(elements.contains(&MorseElement::Dash));
    }

    #[test]
    fn test_audio_generation() {
        let config = CwConfig::new(20, 600.0, 8000.0); // Lower sample rate for test
        let generator = CwGenerator::new(config);
        
        let audio = generator.generate_cw_audio("E"); // Single dot
        
        // Should generate some audio samples
        assert!(!audio.is_empty());
        
        // Should have both tone and silence
        let has_tone = audio.iter().any(|&s| s.abs() > 0.1);
        assert!(has_tone);
    }

    #[test]
    fn test_preamble_generation() {
        let config = CwConfig::new(20, 600.0, 8000.0);
        let generator = CwGenerator::new(config);
        
        let preamble = generator.generate_preamble("N0CALL", "PSK31", Some(14070000.0));
        
        // Should generate a substantial preamble
        assert!(preamble.len() > 1000);
    }
}