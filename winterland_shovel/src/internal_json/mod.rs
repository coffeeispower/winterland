//! Parsing support for Nix's `--log-format internal-json` output.

pub mod parser;
pub mod progress;
pub mod types;

pub use progress::{Progress, ProgressKind};
