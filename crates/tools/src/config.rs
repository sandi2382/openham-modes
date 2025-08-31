//! Configuration management for OpenHam tools

use clap::ArgMatches;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Transmitter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxConfig {
    pub mode: String,
    pub output_file: PathBuf,
    pub input_source: InputSource,
    pub callsign: String,
    pub sample_rate: u32,
    pub carrier_frequency: f64,
    pub verbose: u8,
}

/// Input source for transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputSource {
    Text(String),
    File(PathBuf),
}

impl TxConfig {
    /// Create configuration from command line arguments
    pub fn from_args(matches: &ArgMatches) -> Result<Self> {
        let mode = matches.get_one::<String>("mode")
            .unwrap_or(&"ohm.text.v1".to_string())
            .clone();
        
        let output_file = PathBuf::from(
            matches.get_one::<String>("output")
                .context("Output file is required")?
        );
        
        let input_source = if let Some(text) = matches.get_one::<String>("text") {
            InputSource::Text(text.clone())
        } else if let Some(file) = matches.get_one::<String>("file") {
            InputSource::File(PathBuf::from(file))
        } else {
            anyhow::bail!("Either --text or --file must be specified");
        };
        
        let callsign = matches.get_one::<String>("callsign")
            .context("Callsign is required")?
            .clone();
        
        let sample_rate = matches.get_one::<String>("sample-rate")
            .unwrap_or(&"48000".to_string())
            .parse::<u32>()
            .context("Invalid sample rate")?;
        
        let carrier_frequency = matches.get_one::<String>("frequency")
            .unwrap_or(&"1000".to_string())
            .parse::<f64>()
            .context("Invalid carrier frequency")?;
        
        let verbose = matches.get_count("verbose");
        
        Ok(Self {
            mode,
            output_file,
            input_source,
            callsign,
            sample_rate,
            carrier_frequency,
            verbose,
        })
    }
    
    /// Load configuration from TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        let config: Self = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }
    
    /// Save configuration to TOML file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        
        Ok(())
    }
}

/// Receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RxConfig {
    pub mode: String,
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
    pub sample_rate: u32,
    pub carrier_frequency: f64,
    pub verbose: u8,
}

/// Analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub input_file: PathBuf,
    pub output_file: Option<PathBuf>,
    pub analysis_type: AnalysisType,
    pub sample_rate: u32,
    pub verbose: u8,
}

/// Types of analysis to perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    Spectrum,
    Waterfall,
    Constellation,
    EyeDiagram,
    All,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_serialization() {
        let config = TxConfig {
            mode: "ohm.text.v1".to_string(),
            output_file: PathBuf::from("test.wav"),
            input_source: InputSource::Text("Hello".to_string()),
            callsign: "S56SPZ".to_string(),
            sample_rate: 48000,
            carrier_frequency: 1000.0,
            verbose: 1,
        };
        
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        
        config.save_to_file(&path).unwrap();
        let loaded = TxConfig::from_file(&path).unwrap();
        
        assert_eq!(config.mode, loaded.mode);
        assert_eq!(config.callsign, loaded.callsign);
        assert_eq!(config.sample_rate, loaded.sample_rate);
    }
}