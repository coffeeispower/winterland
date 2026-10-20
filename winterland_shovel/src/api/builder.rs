use crate::{DependencyGraph, Node, OutputError, ShovelConfig};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// A builder for one derivation/output selected from an output tree.
///
/// A builder is the entrypoint for working with a derivation: it identifies the
/// selected output, provides the Nix flake URL for it, and is used to inspect
/// its derivation graph, start its build, and follow build progress.
///
/// A builder is initialized from a derivation node and the configuration for
/// the flake that contains it.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct Builder<'tree> {
    derivation_node: &'tree Node<'tree>,
    output_path: String,
    flake_path: PathBuf,
}

impl<'tree> Builder<'tree> {
    /// Creates a builder for a derivation node in the given flake configuration.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn init(output: &'tree Node<'tree>, config: &ShovelConfig) -> Result<Self, OutputError> {
        let output_path = output.build_node_name();
        let output_path = output_path
            .strip_prefix("outputs.")
            .unwrap_or(&output_path)
            .to_owned();

        Ok(Self {
            derivation_node: output,
            output_path,
            flake_path: PathBuf::from(config.flake_path()),
        })
    }
    /*

        /// Finds an output path in a tree and creates a builder for its node.
        pub fn from_tree(
            config: &ShovelConfig,
            tree: &'tree Tree,
            path: &[&str],
        ) -> Result<Self, OutputError> {
            let output = tree
                .find(path)
                .ok_or_else(|| OutputError::OutputNotFound(path.join(".")))?;
            Self::init(output, config)
        }

    */
    /// Returns the derivation node selected by this builder.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn derivation_node(&self) -> &Node<'tree> {
        self.derivation_node
    }

    /// Returns the dot-separated output path represented by the derivation.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn output_path(&self) -> &str {
        &self.output_path
    }

    /// Returns the absolute path of the flake containing the derivation.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn flake_path(&self) -> &Path {
        &self.flake_path
    }

    /// Returns the flake URL used to refer to this output in Nix commands.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn get_flake_url(&self) -> String {
        format!("path:{}#{}", self.flake_path.display(), self.output_path)
    }

    /// Evaluates this output and returns the complete derivation dependency
    /// graph that Nix would need to build.
    ///
    /// This only evaluates the flake and inspects its derivations. It does not
    /// build any store paths.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb())]
    pub async fn show(&self) -> Result<DependencyGraph, OutputError> {
        let flake_url = self.get_flake_url();
        let mut child = Command::new("nix")
            .args(["derivation", "show", "--recursive", &flake_url])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| OutputError::NixFailed {
                status: None,
                stderr: error.to_string(),
            })?;

        let mut stdout = child.stdout.take().ok_or_else(|| OutputError::NixFailed {
            status: None,
            stderr: "nix stdout was not captured".to_owned(),
        })?;
        let mut stderr = child.stderr.take().ok_or_else(|| OutputError::NixFailed {
            status: None,
            stderr: "nix stderr was not captured".to_owned(),
        })?;
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();

        let (stdout_result, stderr_result, status_result) = tokio::join!(
            stdout.read_to_end(&mut stdout_bytes),
            stderr.read_to_end(&mut stderr_bytes),
            child.wait(),
        );

        let status = status_result.map_err(|error| OutputError::NixFailed {
            status: None,
            stderr: error.to_string(),
        })?;
        stdout_result.map_err(|error| OutputError::NixFailed {
            status: status.code(),
            stderr: error.to_string(),
        })?;
        stderr_result.map_err(|error| OutputError::NixFailed {
            status: status.code(),
            stderr: error.to_string(),
        })?;

        let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
        if !status.success() {
            return Err(OutputError::NixFailed {
                status: status.code(),
                stderr,
            });
        }

        DependencyGraph::from_reader(std::io::Cursor::new(stdout_bytes))
            .map_err(OutputError::InvalidNixOutput)
    }
}
