//! OpenHam TX - Transmit tool for OpenHam digital modes

use clap::Parser;
use anyhow::Result;
use openham_tools::{TxConfig, Transmitter};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .init();

    let mut config = TxConfig::parse();
    
    // Set some text if none provided
    if config.text.is_none() && config.file.is_none() {
        config.text = Some("Hello OpenHam!".to_string());
    }
    
    println!("OpenHam TX starting...");
    
    let mut transmitter = Transmitter::new(config)?;
    let _samples = transmitter.transmit()?;
    
    // TODO: Implement actual audio output
    println!("Transmission complete");
    
    Ok(())
}