//! C binding generation helpers

use anyhow::Result;

/// Generate C bindings for the core library
pub fn generate_bindings() -> Result<()> {
    // TODO: Implement cbindgen integration
    // This would use cbindgen to generate C headers
    
    println!("C binding generation not yet implemented");
    println!("Would generate headers for:");
    println!("  - openham_core.h");
    println!("  - openham_frame.h");
    println!("  - openham_codecs.h");
    println!("  - openham_modem.h");
    
    Ok(())
}

/// Configuration for cbindgen
pub struct CBindgenConfig {
    pub crate_dir: String,
    pub output_file: String,
    pub language: BindingLanguage,
}

/// Supported binding languages
pub enum BindingLanguage {
    C,
    Cpp,
}

impl CBindgenConfig {
    pub fn new_c(crate_dir: &str, output_file: &str) -> Self {
        Self {
            crate_dir: crate_dir.to_string(),
            output_file: output_file.to_string(),
            language: BindingLanguage::C,
        }
    }
    
    pub fn new_cpp(crate_dir: &str, output_file: &str) -> Self {
        Self {
            crate_dir: crate_dir.to_string(),
            output_file: output_file.to_string(),
            language: BindingLanguage::Cpp,
        }
    }
}