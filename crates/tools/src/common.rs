//! Common utilities and configuration for tools

use clap::Parser;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

/// Global configuration options
#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub struct GlobalConfig {
    /// Configuration file path
    #[arg(long)]
    pub config: Option<PathBuf>,
    
    /// Enable debug output
    #[arg(long)]
    pub debug: bool,
    
    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,
    
    /// Working directory
    #[arg(long)]
    pub work_dir: Option<PathBuf>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            config: None,
            debug: false,
            log_level: "info".to_string(),
            work_dir: None,
        }
    }
}

/// Audio file format detection and handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Raw,
    Complex,
}

impl AudioFormat {
    /// Detect format from file extension
    pub fn from_path(path: &PathBuf) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("wav") => AudioFormat::Wav,
            Some("raw") => AudioFormat::Raw,
            Some("iq") | Some("complex") => AudioFormat::Complex,
            _ => AudioFormat::Raw, // Default
        }
    }
    
    /// Get file extension for format
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Raw => "raw",
            AudioFormat::Complex => "iq",
        }
    }
}

/// Sample format for raw audio files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    F32Le,
    F64Le,
    I16Le,
    I32Le,
}

impl SampleFormat {
    /// Get bytes per sample
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            SampleFormat::F32Le => 4,
            SampleFormat::F64Le => 8,
            SampleFormat::I16Le => 2,
            SampleFormat::I32Le => 4,
        }
    }
    
    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "f32" | "f32le" => Ok(SampleFormat::F32Le),
            "f64" | "f64le" => Ok(SampleFormat::F64Le),
            "i16" | "i16le" => Ok(SampleFormat::I16Le),
            "i32" | "i32le" => Ok(SampleFormat::I32Le),
            _ => anyhow::bail!("Unknown sample format: {}", s),
        }
    }
}

/// Progress reporter for long-running operations
pub struct ProgressReporter {
    total: usize,
    current: usize,
    last_percent: u8,
    verbose: bool,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(total: usize, verbose: bool) -> Self {
        Self {
            total,
            current: 0,
            last_percent: 0,
            verbose,
        }
    }
    
    /// Update progress
    pub fn update(&mut self, current: usize) {
        self.current = current;
        
        if self.verbose && self.total > 0 {
            let percent = ((self.current * 100) / self.total) as u8;
            if percent != self.last_percent && percent % 10 == 0 {
                println!("Progress: {}%", percent);
                self.last_percent = percent;
            }
        }
    }
    
    /// Mark as complete
    pub fn complete(&mut self) {
        if self.verbose {
            println!("Complete: {}/{} (100%)", self.current, self.total);
        }
    }
}

/// Initialize logging based on configuration
pub fn init_logging(config: &GlobalConfig) -> Result<()> {
    // Simple logging setup for now
    if config.debug {
        println!("Debug logging enabled");
    }
    
    Ok(())
}

/// Load configuration from file
pub fn load_config<T: for<'a> Deserialize<'a>>(path: &PathBuf) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    
    // Try JSON first, then TOML
    if let Ok(config) = serde_json::from_str(&content) {
        return Ok(config);
    }
    
    // Try TOML
    match toml::from_str(&content) {
        Ok(config) => Ok(config),
        Err(e) => anyhow::bail!("Failed to parse config file: {}", e),
    }
}

/// Save configuration to file
pub fn save_config<T: Serialize>(config: &T, path: &PathBuf) -> Result<()> {
    let content = if path.extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::to_string_pretty(config)?
    } else {
        toml::to_string_pretty(config)?
    };
    
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_detection() {
        assert_eq!(AudioFormat::from_path(&PathBuf::from("test.wav")), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_path(&PathBuf::from("test.raw")), AudioFormat::Raw);
        assert_eq!(AudioFormat::from_path(&PathBuf::from("test.iq")), AudioFormat::Complex);
        assert_eq!(AudioFormat::from_path(&PathBuf::from("test.unknown")), AudioFormat::Raw);
    }

    #[test]
    fn test_sample_format() {
        assert_eq!(SampleFormat::from_str("f32").unwrap(), SampleFormat::F32Le);
        assert_eq!(SampleFormat::from_str("I16LE").unwrap(), SampleFormat::I16Le);
        assert!(SampleFormat::from_str("unknown").is_err());
    }

    #[test]
    fn test_progress_reporter() {
        let mut reporter = ProgressReporter::new(100, false);
        reporter.update(50);
        assert_eq!(reporter.current, 50);
        reporter.complete();
    }
}