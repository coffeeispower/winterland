use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct Document {
    pub(super) derivations: BTreeMap<String, Derivation>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Metadata {
    pub(super) source: Option<String>,
    #[serde(rename = "displayName")]
    pub(super) display_name: Option<String>,
    pub(super) website: Option<String>,
    pub(super) repository: Option<String>,
    pub(super) version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Derivation {
    pub(super) args: Vec<String>,
    pub(super) builder: String,
    pub(super) env: BTreeMap<String, String>,
    pub(super) inputs: Inputs,
    pub(super) name: String,
    pub(super) outputs: BTreeMap<String, Output>,
    pub(super) system: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Inputs {
    pub(super) drvs: BTreeMap<String, InputDerivation>,
    pub(super) srcs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct InputDerivation {
    pub(super) outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Output {
    pub(super) hash: Option<String>,
    pub(super) method: Option<String>,
    pub(super) path: Option<String>,
}

impl From<Output> for super::DerivationOutput {
    fn from(output: Output) -> Self {
        Self {
            hash: output.hash,
            method: output.method,
            path: output.path,
        }
    }
}

impl From<Derivation> for super::Derivation {
    fn from(derivation: Derivation) -> Self {
        Self {
            args: derivation.args,
            builder: derivation.builder,
            environment: derivation.env,
            inputs: derivation
                .inputs
                .drvs
                .into_iter()
                .map(|(path, input)| super::DerivationInput {
                    derivation: path,
                    outputs: input.outputs,
                })
                .collect(),
            source_inputs: derivation.inputs.srcs,
            name: derivation.name,
            source_location: None,
            display_name: None,
            website: None,
            repository: None,
            version: None,
            outputs: derivation
                .outputs
                .into_iter()
                .map(|(name, output)| (name, output.into()))
                .collect(),
            system: derivation.system,
        }
    }
}
