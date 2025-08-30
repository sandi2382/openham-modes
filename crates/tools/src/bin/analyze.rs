//! OpenHam Analyze - Signal analysis tool for OpenHam digital modes

use clap::Parser;
use anyhow::Result;
use openham_tools::{AnalyzeConfig, SignalAnalyzer};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .init();

    let config = AnalyzeConfig::parse();
    
    println!("OpenHam Analyze starting...");
    
    let _analyzer = SignalAnalyzer::new(config)?;
    
    // TODO: Implement actual signal analysis
    println!("Analyzer created successfully");
    
    Ok(())
}