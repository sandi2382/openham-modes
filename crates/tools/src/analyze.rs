//! Signal analysis tools and utilities

use clap::Parser;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

use openham_core::buffer::Complex;
use openham_core::prelude::*;

/// Analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
#[command(name = "analyze")]
#[command(about = "OpenHam signal analysis tool")]
pub struct AnalyzeConfig {
    /// Input file path (audio samples)
    #[arg(short, long)]
    pub input: PathBuf,
    
    /// Output file path (analysis results)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// Sample rate in Hz
    #[arg(long, default_value = "48000")]
    pub sample_rate: f64,
    
    /// FFT size for spectral analysis
    #[arg(long, default_value = "1024")]
    pub fft_size: usize,
    
    /// Analysis window size in samples
    #[arg(long, default_value = "4096")]
    pub window_size: usize,
    
    /// Enable spectral analysis
    #[arg(long)]
    pub spectral: bool,
    
    /// Enable constellation analysis
    #[arg(long)]
    pub constellation: bool,
    
    /// Enable waterfall display
    #[arg(long)]
    pub waterfall: bool,
    
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            input: PathBuf::from("input.wav"),
            output: None,
            sample_rate: 48000.0,
            fft_size: 1024,
            window_size: 4096,
            spectral: false,
            constellation: false,
            waterfall: false,
            verbose: false,
        }
    }
}

/// Signal analyzer
pub struct SignalAnalyzer {
    config: AnalyzeConfig,
    fft_processor: FftProcessor,
}

impl SignalAnalyzer {
    /// Create a new signal analyzer
    pub fn new(config: AnalyzeConfig) -> Result<Self> {
        let fft_config = FftConfig::new(config.fft_size, config.sample_rate)?;
        let fft_processor = FftProcessor::new(fft_config)?;
        
        Ok(Self {
            config,
            fft_processor,
        })
    }
    
    /// Analyze signal samples
    pub fn analyze(&mut self, samples: &[Complex]) -> Result<AnalysisResult> {
        if self.config.verbose {
            println!("Analyzing {} samples", samples.len());
        }
        
        let mut result = AnalysisResult::default();
        
        // Basic signal statistics
        result.sample_count = samples.len();
        result.power = self.calculate_power(samples);
        result.peak_amplitude = self.calculate_peak_amplitude(samples);
        
        // Spectral analysis
        if self.config.spectral {
            result.spectrum = Some(self.compute_spectrum(samples)?);
        }
        
        // Constellation analysis
        if self.config.constellation {
            result.constellation = Some(self.compute_constellation(samples));
        }
        
        if self.config.verbose {
            println!("Analysis complete: power={:.2} dB, peak={:.4}", 
                    10.0 * result.power.log10(), result.peak_amplitude);
        }
        
        Ok(result)
    }
    
    fn calculate_power(&self, samples: &[Complex]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        
        let sum: f64 = samples.iter()
            .map(|s| s.norm_sqr())
            .sum();
        
        sum / samples.len() as f64
    }
    
    fn calculate_peak_amplitude(&self, samples: &[Complex]) -> f64 {
        samples.iter()
            .map(|s| s.norm())
            .fold(0.0, f64::max)
    }
    
    fn compute_spectrum(&mut self, samples: &[Complex]) -> Result<Vec<f64>> {
        // Use windowed FFT for spectrum computation
        let window_size = self.config.fft_size.min(samples.len());
        let window_samples = &samples[..window_size];
        
        let mut fft_input = vec![Complex::new(0.0, 0.0); self.config.fft_size];
        fft_input[..window_samples.len()].copy_from_slice(window_samples);
        
        // Apply window function (Hanning)
        for (i, sample) in fft_input.iter_mut().enumerate() {
            let window_val = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (self.config.fft_size - 1) as f64).cos());
            *sample = *sample * window_val;
        }
        
        // Convert to power spectrum
        let mut fft_output = vec![Complex::new(0.0, 0.0); self.config.fft_size];
        self.fft_processor.fft(&fft_input, &mut fft_output)?;
        
        let spectrum = fft_output.iter()
            .map(|c| c.norm_sqr())
            .collect();
        
        Ok(spectrum)
    }
    
    fn compute_constellation(&self, samples: &[Complex]) -> Vec<(f64, f64)> {
        // Downsample for constellation display
        let step = (samples.len() / 1000).max(1);
        samples.iter()
            .step_by(step)
            .map(|c| (c.real, c.imag))
            .collect()
    }
}

/// Analysis results
#[derive(Debug, Default)]
pub struct AnalysisResult {
    pub sample_count: usize,
    pub power: f64,
    pub peak_amplitude: f64,
    pub spectrum: Option<Vec<f64>>,
    pub constellation: Option<Vec<(f64, f64)>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_config_default() {
        let config = AnalyzeConfig::default();
        assert_eq!(config.sample_rate, 48000.0);
        assert_eq!(config.fft_size, 1024);
        assert_eq!(config.window_size, 4096);
    }

    #[test]
    fn test_analyzer_creation() {
        let config = AnalyzeConfig::default();
        let _analyzer = SignalAnalyzer::new(config).unwrap();
    }

    #[test]
    fn test_power_calculation() {
        let config = AnalyzeConfig::default();
        let analyzer = SignalAnalyzer::new(config).unwrap();
        
        let samples = vec![
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 1.0),
            Complex::new(-1.0, 0.0),
            Complex::new(0.0, -1.0),
        ];
        
        let power = analyzer.calculate_power(&samples);
        assert!((power - 1.0).abs() < 1e-10);
    }
}