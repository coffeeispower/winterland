use std::path::{Path, PathBuf};

/// Errors encountered while creating a [`ShovelConfig`].
#[derive(Debug)]
pub enum ConfigError {
    FlakeNotFound(std::io::Error),
    InvalidNixFile,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FlakeNotFound(error) => write!(f, "could not resolve flake.nix: {error}"),
            Self::InvalidNixFile => write!(f, "the flake path must have a .nix extension"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration for inspecting and building outputs from a Nix flake.
///
/// The configuration resolves the supplied flake path to an absolute path and
/// is shared by output-tree queries and builders created from that tree.
#[derive(Debug, Clone)]
pub struct ShovelConfig {
    flake_path: PathBuf,
}

impl ShovelConfig {
    /// Creates a configuration for `path`, defaulting to `./flake.nix`.
    ///
    /// The path must refer to a file with a `.nix` extension and is stored as
    /// its canonical absolute path.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn new(path: Option<String>) -> Result<Self, ConfigError> {
        let path = path
            .as_ref()
            .map(Path::new)
            .unwrap_or_else(|| Path::new("flake.nix"));

        if path.extension().and_then(|extension| extension.to_str()) != Some("nix") {
            return Err(ConfigError::InvalidNixFile);
        }

        let flake_path = std::fs::canonicalize(path).map_err(ConfigError::FlakeNotFound)?;
        Ok(Self { flake_path })
    }

    /// Returns the canonical absolute path of the configured flake file.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn flake_path(&self) -> String {
        self.flake_path.display().to_string()
    }
}
