use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;

/// A normalized dependency graph for the derivations needed to build an
/// output.
///
/// This is Winterland's domain model rather than a representation of any
/// particular Nix command's JSON format. Nix-specific fields are parsed by
/// private wire-format types and converted into this graph at the boundary.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct DependencyGraph {
    /// Derivations keyed by their Nix `.drv` store-path suffix.
    derivations: BTreeMap<String, Derivation>,
}

impl DependencyGraph {
    /// Deserializes and normalizes the output of
    /// `nix derivation show --recursive` from a reader.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, serde_json::Error> {
        let document: nix::Document = serde_json::from_reader(reader)?;

        Ok(Self {
            derivations: document
                .derivations
                .into_iter()
                .map(|(path, derivation)| (path, derivation.into()))
                .collect(),
        })
    }
}

/// The Winterland representation of one build derivation.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct Derivation {
    /// Arguments passed to the derivation builder.
    args: Vec<String>,
    /// Store path of the executable used to build the derivation.
    builder: String,
    /// Environment variables supplied to the derivation.
    environment: BTreeMap<String, String>,
    /// Derivations and outputs required to build this derivation.
    inputs: Vec<DerivationInput>,
    /// Source store paths consumed by the derivation.
    source_inputs: Vec<String>,
    /// Human-readable derivation name.
    name: String,
    /// Outputs declared by this derivation, keyed by output name.
    outputs: BTreeMap<String, DerivationOutput>,
    /// System for which the derivation is built.
    system: String,
}

/// A dependency on outputs from another derivation.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct DerivationInput {
    /// The `.drv` store-path suffix of the dependency.
    derivation: String,
    /// Outputs required from the dependency.
    outputs: Vec<String>,
}

/// Metadata for one output produced by a derivation.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct DerivationOutput {
    /// Content hash, when the output is content-addressed.
    hash: Option<String>,
    /// Hashing method, when the output is content-addressed.
    method: Option<String>,
    /// Output path relative to the Nix store hash, when known.
    path: Option<String>,
}

/// Private types matching the JSON emitted by `nix derivation show`.
///
/// These types intentionally do not form part of Winterland's public API.
/// Nix may add or change fields as its output format evolves without forcing
/// those implementation details into the rest of the application.
mod nix;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_derivation_show_recursive_output() {
        let graph = DependencyGraph::from_reader(
            &include_bytes!("derivations/fixtures/derivation.json")[..],
        )
        .unwrap();

        assert!(!graph.derivations.is_empty());
        let derivation = graph.derivations.values().next().unwrap();
        assert!(!derivation.name.is_empty());
        assert!(!derivation.outputs.is_empty());
        assert!(!derivation.inputs.is_empty());
    }
}
