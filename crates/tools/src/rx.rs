//! Receiver configuration and implementation

use clap::Parser;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

use openham_core::buffer::Complex;
use openham_modem::prelude::*;
use openham_frame::prelude::*;
use openham_codecs::prelude::*;

/// Receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
#[command(name = "rx")]
#[command(about = "OpenHam receiver tool")]
pub struct RxConfig {
    /// Input file path (audio samples)
    #[arg(short, long)]
    pub input: PathBuf,
    
    /// Output file path (decoded text)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
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

impl Default for RxConfig {
    fn default() -> Self {
        Self {
            input: PathBuf::from("input.wav"),
            output: None,
            sample_rate: 48000.0,
            center_freq: 1500.0,
            symbol_rate: 125.0,
            modulation: "bpsk".to_string(),
            codec: "huffman".to_string(),
            verbose: false,
        }
    }
}

/// OpenHam receiver
pub struct Receiver {
    config: RxConfig,
    demodulator: Box<dyn openham_modem::common::Demodulator>,
    codec_registry: CodecRegistry,
}

impl Receiver {
    /// Create a new receiver with the given configuration
    pub fn new(config: RxConfig) -> Result<Self> {
        // Create modulation configuration
        let mod_config = ModulationConfig::new(
            config.sample_rate,
            config.symbol_rate,
            config.center_freq,
        )?;
        
        // Create demodulator based on configuration
        let demodulator: Box<dyn openham_modem::common::Demodulator> = match config.modulation.as_str() {
            "bpsk" => Box::new(BpskDemodulator::new(mod_config)?),
            _ => anyhow::bail!("Unsupported modulation scheme: {}", config.modulation),
        };
        
        // Create codec registry
        let codec_registry = CodecRegistry::new();
        
        Ok(Self {
            config,
            demodulator,
            codec_registry,
        })
    }
    
    /// Receive and decode data from input samples
    pub fn receive(&mut self, samples: &[Complex]) -> Result<Option<String>> {
        if self.config.verbose {
            println!("Processing {} samples", samples.len());
        }
        
        // Demodulate samples to bits
        let mut bits = Vec::new();
        self.demodulator.demodulate(samples, &mut bits)?;
        
        if bits.is_empty() {
            return Ok(None);
        }
        
        // Try to decode frame
        match Frame::from_bytes(&bits) {
            Ok(frame) => {
                if self.config.verbose {
                    println!("Decoded frame with {} payload bytes", frame.payload.len());
                }
                
                // Decode payload using specified codec
                let text = match self.config.codec.as_str() {
                    "huffman" => {
                        let mut codec = openham_codecs::text::HuffmanCodec::new_english();
                        codec.decode(&frame.payload)?
                    },
                    "ascii" => String::from_utf8(frame.payload.clone())?,
                    _ => anyhow::bail!("Unknown codec: {}", self.config.codec),
                };
                Ok(Some(text))
            },
            Err(e) => {
                if self.config.verbose {
                    println!("Failed to decode frame: {}", e);
                }
                Ok(None)
            }
        }
    }
    
    /// Get signal quality metrics
    pub fn signal_quality(&self) -> openham_modem::common::SignalQuality {
        self.demodulator.signal_quality()
    }
    
    /// Check if receiver is synchronized
    pub fn is_synchronized(&self) -> bool {
        self.demodulator.is_synchronized()
    }
    
    /// Reset receiver state
    pub fn reset(&mut self) {
        self.demodulator.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rx_config_default() {
        let config = RxConfig::default();
        assert_eq!(config.sample_rate, 48000.0);
        assert_eq!(config.center_freq, 1500.0);
        assert_eq!(config.symbol_rate, 125.0);
        assert_eq!(config.modulation, "bpsk");
        assert_eq!(config.codec, "huffman");
    }

    #[test]
    fn test_receiver_creation() {
        let config = RxConfig::default();
        let _receiver = Receiver::new(config).unwrap();
    }
}