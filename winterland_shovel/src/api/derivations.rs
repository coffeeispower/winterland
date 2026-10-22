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
    /// Returns the number of derivations in the graph.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn derivation_count(&self) -> usize {
        self.derivations.len()
    }

    /// Returns the `.drv` paths that identify derivations in this graph.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn derivation_paths(&self) -> Vec<String> {
        self.derivations.keys().cloned().collect()
    }

    /// Looks up a derivation by its `.drv` store path.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn derivation(&self, path: &str) -> Option<Derivation> {
        self.derivations.get(path).cloned()
    }

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

    /// Adds metadata collected by re-evaluating the selected Nix package.
    fn apply_metadata(&mut self, metadata: BTreeMap<String, nix::Metadata>) {
        for (path, metadata) in metadata {
            if let Some(derivation) = self.derivations.get_mut(&path) {
                derivation.source_location = metadata.source;
                derivation.display_name = metadata.display_name;
                derivation.website = metadata.website;
                derivation.repository = metadata.repository;
                derivation.version = metadata.version;
            }
        }
    }

    /// Reads and applies the JSON mapping produced by the metadata evaluation.
    pub(crate) fn apply_metadata_reader<R: Read>(
        &mut self,
        reader: R,
    ) -> Result<(), serde_json::Error> {
        let metadata = serde_json::from_reader(reader)?;
        self.apply_metadata(metadata);
        Ok(())
    }
}

/// The Winterland representation of one build derivation.
#[derive(Debug, Clone)]
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
    /// Source location reported by the evaluated package metadata.
    source_location: Option<String>,
    /// Optional Winterland-specific display name.
    display_name: Option<String>,
    /// Website associated with the package, when provided by its metadata.
    website: Option<String>,
    /// Source repository associated with the package, such as a GitHub URL.
    repository: Option<String>,
    /// Package version reported by the evaluated derivation.
    version: Option<String>,
    /// Outputs declared by this derivation, keyed by output name.
    outputs: BTreeMap<String, DerivationOutput>,
    /// System for which the derivation is built.
    system: String,
}

impl Derivation {
    /// Returns the derivation's Nix name.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Returns the builder executable used by this derivation.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn builder(&self) -> String {
        self.builder.clone()
    }

    /// Returns the target system of this derivation.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn system(&self) -> String {
        self.system.clone()
    }

    /// Returns the derivations required by this derivation.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn inputs(&self) -> Vec<DerivationInput> {
        self.inputs.clone()
    }

    /// Returns the source paths consumed by this derivation.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn source_inputs(&self) -> Vec<String> {
        self.source_inputs.clone()
    }

    /// Returns the names of this derivation's outputs.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn output_names(&self) -> Vec<String> {
        self.outputs.keys().cloned().collect()
    }

    /// Looks up one output by name.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn output(&self, name: &str) -> Option<DerivationOutput> {
        self.outputs.get(name).cloned()
    }

    /// Returns the source location associated with this derivation, when one
    /// was available during evaluation.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn source_location(&self) -> Option<String> {
        self.source_location.clone()
    }

    /// Returns the optional Winterland-specific display name.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn display_name(&self) -> Option<String> {
        self.display_name.clone()
    }

    /// Returns the package website, when one was provided by its metadata.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn website(&self) -> Option<String> {
        self.website.clone()
    }

    /// Returns the package source repository, when one was provided by its
    /// metadata.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn repository(&self) -> Option<String> {
        self.repository.clone()
    }

    /// Returns the package version, when one was reported by the derivation.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn version(&self) -> Option<String> {
        self.version.clone()
    }
}

/// A dependency on outputs from another derivation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct DerivationInput {
    /// The `.drv` store-path suffix of the dependency.
    derivation: String,
    /// Outputs required from the dependency.
    outputs: Vec<String>,
}

impl DerivationInput {
    /// Returns the dependency's `.drv` store path.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn derivation(&self) -> String {
        self.derivation.clone()
    }

    /// Returns the outputs required from the dependency.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn outputs(&self) -> Vec<String> {
        self.outputs.clone()
    }
}

/// Metadata for one output produced by a derivation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(opaque))]
pub struct DerivationOutput {
    /// Content hash, when the output is content-addressed.
    hash: Option<String>,
    /// Hashing method, when the output is content-addressed.
    method: Option<String>,
    /// Output path relative to the Nix store hash, when known.
    path: Option<String>,
}

impl DerivationOutput {
    /// Returns the content hash, when this is a content-addressed output.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn hash(&self) -> Option<String> {
        self.hash.clone()
    }

    /// Returns the hashing method, when this is a content-addressed output.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn method(&self) -> Option<String> {
        self.method.clone()
    }

    /// Returns the output path, when known.
    #[cfg_attr(
        feature = "flutter_rust_bridge",
        flutter_rust_bridge::frb(sync, getter)
    )]
    pub fn path(&self) -> Option<String> {
        self.path.clone()
    }
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
