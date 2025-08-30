//! Transmitter configuration and implementation

use clap::Parser;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

use openham_core::buffer::Complex;
use openham_modem::prelude::*;
use openham_frame::prelude::*;
use openham_codecs::prelude::*;

/// Transmitter configuration
#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
#[command(name = "tx")]
#[command(about = "OpenHam transmitter tool")]
pub struct TxConfig {
    /// Output file path (audio samples)
    #[arg(short, long)]
    pub output: PathBuf,
    
    /// Input text to transmit
    #[arg(short, long)]
    pub text: Option<String>,
    
    /// Input file path (text file)
    #[arg(short, long)]
    pub file: Option<PathBuf>,
    
    /// Station callsign
    #[arg(short, long)]
    pub callsign: String,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// Center frequency in Hz
    #[arg(long, default_value = "1500")]
    pub center_freq: f64,
    
    /// Symbol rate in Hz
    #[arg(long, default_value = "125")]
    pub symbol_rate: f64,
    
    /// Modulation scheme
    #[arg(long, default_value = "bpsk")]
    pub modulation: String,
    
    /// Text codec
    #[arg(long, default_value = "huffman")]
    pub codec: String,
    
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

impl Default for TxConfig {
    fn default() -> Self {
        Self {
            output: PathBuf::from("output.wav"),
            text: None,
            file: None,
            callsign: "NOCALL".to_string(),
            sample_rate: 48000.0,
            center_freq: 1500.0,
            symbol_rate: 125.0,
            modulation: "bpsk".to_string(),
            codec: "huffman".to_string(),
            verbose: false,
        }
    }
}

/// OpenHam transmitter
pub struct Transmitter {
    config: TxConfig,
    modulator: Box<dyn openham_modem::common::Modulator>,
    codec_registry: CodecRegistry,
}

impl Transmitter {
    /// Create a new transmitter with the given configuration
    pub fn new(config: TxConfig) -> Result<Self> {
        // Validate configuration
        if config.text.is_none() && config.file.is_none() {
            anyhow::bail!("Either text or file must be specified");
        }
        
        // Create modulation configuration
        let mod_config = ModulationConfig::new(
            config.sample_rate,
            config.symbol_rate,
            config.center_freq,
        )?;
        
        // Create modulator based on configuration
        let modulator: Box<dyn openham_modem::common::Modulator> = match config.modulation.as_str() {
            "bpsk" => Box::new(BpskModulator::new(mod_config)?),
            _ => anyhow::bail!("Unsupported modulation scheme: {}", config.modulation),
        };
        
        // Create codec registry
        let codec_registry = CodecRegistry::new();
        
        Ok(Self {
            config,
            modulator,
            codec_registry,
        })
    }
    
    /// Transmit the configured message
    pub fn transmit(&mut self) -> Result<Vec<Complex>> {
        // Get text to transmit
        let text = if let Some(text) = &self.config.text {
            text.clone()
        } else if let Some(file) = &self.config.file {
            std::fs::read_to_string(file)?
        } else {
            anyhow::bail!("No text or file specified");
        };
        
        if self.config.verbose {
            println!("Transmitting: {}", text);
        }
        
        // Encode text using specified codec
        let encoded_data = match self.config.codec.as_str() {
            "huffman" => {
                let mut codec = openham_codecs::text::HuffmanCodec::new_english();
                codec.encode(&text)?
            },
            "ascii" => text.as_bytes().to_vec(),
            _ => anyhow::bail!("Unknown codec: {}", self.config.codec),
        };
        
        // Create frame (frame_type=1, sequence=0, flags=0)
        let frame = Frame::new(1, 0, encoded_data, 0);
        
        let frame_bytes = frame.to_bytes();
        
        if self.config.verbose {
            println!("Frame size: {} bytes", frame_bytes.len());
        }
        
        // Modulate frame
        let mut samples = Vec::new();
        self.modulator.modulate(&frame_bytes, &mut samples)?;
        
        if self.config.verbose {
            println!("Generated {} samples", samples.len());
        }
        
        Ok(samples)
    }
    
    /// Get samples per symbol
    pub fn samples_per_symbol(&self) -> usize {
        self.modulator.samples_per_symbol()
    }
    
    /// Get symbol rate
    pub fn symbol_rate(&self) -> f64 {
        self.modulator.symbol_rate()
    }
    
    /// Reset transmitter state
    pub fn reset(&mut self) {
        self.modulator.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_config_default() {
        let config = TxConfig::default();
        assert_eq!(config.sample_rate, 48000.0);
        assert_eq!(config.center_freq, 1500.0);
        assert_eq!(config.symbol_rate, 125.0);
        assert_eq!(config.modulation, "bpsk");
        assert_eq!(config.codec, "huffman");
        assert_eq!(config.callsign, "NOCALL");
    }

    #[test]
    fn test_transmitter_creation() {
        let mut config = TxConfig::default();
        config.text = Some("Hello World".to_string());
        config.callsign = "W1AW".to_string();
        
        let _transmitter = Transmitter::new(config).unwrap();
    }
}