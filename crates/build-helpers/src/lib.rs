//! Build helpers and packaging utilities

pub mod cbindgen_helper;
pub mod packaging;
pub mod version;

/// Generate C bindings using cbindgen
pub fn generate_c_bindings() -> anyhow::Result<()> {
    cbindgen_helper::generate_bindings()
}

/// Package release artifacts
pub fn package_release(target: &str, output_dir: &str) -> anyhow::Result<()> {
    packaging::create_release_package(target, output_dir)
}

/// Get version information
pub fn get_version_info() -> version::VersionInfo {
    version::get_build_info()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = get_version_info();
        assert!(!info.version.is_empty());
    }
}