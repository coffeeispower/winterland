#[cfg(feature = "flutter_rust_bridge")]
mod frb_generated;
mod internal_json;
mod api {
    pub mod builder;
    pub mod config;
    pub mod derivations;
    mod nix_command;
    pub mod outputs;

    pub use builder::Builder;
    pub use config::{ConfigError, ShovelConfig};
    pub use derivations::{DependencyGraph, Derivation, DerivationInput, DerivationOutput};
    pub use outputs::{Kind, Node, OutputError, Tree};
}
pub use api::*;
pub use internal_json::{Progress, ProgressKind};
