//! Version information utilities

use serde::{Deserialize, Serialize};

/// Build and version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_hash: Option<String>,
    pub build_date: String,
    pub rust_version: String,
    pub target: String,
}

/// Get comprehensive version information
pub fn get_build_info() -> VersionInfo {
    VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_hash: option_env!("GIT_HASH").map(|s| s.to_string()),
        build_date: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        rust_version: std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
        target: std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string()),
    }
}

/// Get simple version string
pub fn get_version_string() -> String {
    let info = get_build_info();
    if let Some(git_hash) = info.git_hash {
        format!("{} ({})", info.version, &git_hash[..8])
    } else {
        info.version
    }
}