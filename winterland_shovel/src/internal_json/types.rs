//! Generic types for Nix internal JSON activity records.

use serde::Deserialize;

/// A field in a Nix activity record.
///
/// Nix serializes activity fields as either numbers or strings depending on
/// the activity type. Keeping that distinction avoids converting byte
/// counters to text before the parser interprets them.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Field {
    /// An unsigned numeric field.
    Number(u64),
    /// A textual field.
    Text(String),
}

/// One `@nix {...}` activity record emitted by Nix.
///
/// The activity envelope is shared by multiple Nix commands, while the
/// meaning of `event_type`, `fields`, and `text` depends on the command and
/// Nix version. Unknown fields are intentionally ignored so callers can
/// continue parsing newer records.
#[derive(Debug, Deserialize)]
pub struct Event {
    /// Activity transition, such as `start`, `stop`, or `result`.
    pub action: String,
    /// Identifier used to correlate activity records.
    pub id: Option<u64>,
    /// Parent activity identifier, when present.
    #[serde(default)]
    pub parent: Option<u64>,
    /// Internal activity scope identifier, when present.
    #[serde(default)]
    pub sid: Option<String>,
    /// Nix logging level, when present.
    #[serde(default)]
    pub level: Option<u64>,
    /// Command-specific activity type number, when present.
    #[serde(rename = "type", default)]
    pub event_type: Option<u64>,
    /// Human-readable activity text, when present.
    #[serde(default)]
    pub text: Option<String>,
    /// Command-specific fields, which may be numbers or strings.
    #[serde(default)]
    pub fields: Option<Vec<Field>>,
}

/// A parsed and interpreted message from Nix's internal JSON log.
#[derive(Debug)]
pub enum Message {
    /// Nix started downloading or copying an input.
    InputDownloadStarted {
        id: Option<u64>,
        description: String,
    },
    /// Nix finished downloading or copying an input.
    InputDownloadFinished {
        id: Option<u64>,
        description: String,
    },
    /// Nix reported byte-level progress for an input download.
    DownloadProgress {
        id: u64,
        description: String,
        done: u64,
        expected: u64,
        running: u64,
        failed: u64,
    },
    /// Nix reported the expected size of an input download.
    DownloadExpected {
        id: u64,
        description: String,
        expected: u64,
    },
    /// Nix started evaluating an output path.
    EvaluationStarted { id: Option<u64>, path: String },
    /// A generic activity started and was not recognized more specifically.
    ActivityStarted {
        id: Option<u64>,
        description: Option<String>,
    },
    /// A generic activity stopped and was not recognized more specifically.
    ActivityStopped { id: Option<u64> },
    /// Nix emitted a command-specific result record.
    ActivityResult {
        id: Option<u64>,
        event_type: Option<u64>,
        fields: Option<Vec<Field>>,
    },
    /// An ordinary, non-structured stderr line.
    Text(String),
}
