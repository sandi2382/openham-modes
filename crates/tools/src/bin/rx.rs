//! OpenHam RX - Receive tool for OpenHam digital modes

use clap::Parser;
use anyhow::Result;
use openham_tools::{RxConfig, Receiver};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .init();

    let config = RxConfig::parse();
    
    println!("OpenHam RX starting...");
    
    let mut receiver = Receiver::new(config)?;
    
    // TODO: Implement actual audio input and processing
    println!("Receiver created successfully");
    
    Ok(())
}