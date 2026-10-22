use crate::internal_json::parser;
use crate::{DependencyGraph, Tree};
use serde::{Deserialize, Serialize};

/// The phase of a query progress event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb())]
pub enum ProgressKind {
    /// Nix started downloading or copying an input.
    InputDownloadStarted,
    /// Nix finished downloading or copying an input.
    InputDownloadFinished,
    /// Nix reported byte-level download progress for an input.
    InputDownloadProgress,
    /// Nix started evaluating an output path in the flake.
    EvaluationStarted,
    /// Nix finished querying and the event contains a result.
    Completed,
}

/// One progress event emitted while running a Nix query.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb())]
pub struct Progress {
    /// The phase transition represented by this event.
    pub kind: ProgressKind,
    /// The input description or output path associated with the event.
    pub value: String,
    /// Number of bytes downloaded, when this is a byte-progress event.
    pub done: Option<u64>,
    /// Expected number of bytes, when known.
    pub expected: Option<u64>,
    /// Number of active transfers reported by Nix.
    pub running: Option<u64>,
    /// Number of failed transfers reported by Nix.
    pub failed: Option<u64>,
    /// The completed output tree, when this is a tree query.
    pub tree: Option<Tree>,
    /// The completed derivation graph, when this is a derivation query.
    pub graph: Option<DependencyGraph>,
}

impl Progress {
    pub(crate) fn phase(kind: ProgressKind, value: String) -> Self {
        Self {
            kind,
            value,
            done: None,
            expected: None,
            running: None,
            failed: None,
            tree: None,
            graph: None,
        }
    }

    pub(crate) fn from_event(progress: parser::ProgressEvent) -> Option<Self> {
        Some(match progress {
            parser::ProgressEvent::InputDownloadStarted(description) => {
                Self::phase(ProgressKind::InputDownloadStarted, description)
            }
            parser::ProgressEvent::InputDownloadFinished(description) => {
                Self::phase(ProgressKind::InputDownloadFinished, description)
            }
            parser::ProgressEvent::DownloadProgress {
                description,
                done,
                expected,
                running,
                failed,
            } => Self {
                kind: ProgressKind::InputDownloadProgress,
                value: description,
                done: Some(done),
                expected: Some(expected),
                running: Some(running),
                failed: Some(failed),
                tree: None,
                graph: None,
            },
            parser::ProgressEvent::DownloadExpected {
                description,
                expected,
            } => Self {
                kind: ProgressKind::InputDownloadProgress,
                value: description,
                done: None,
                expected: Some(expected),
                running: None,
                failed: None,
                tree: None,
                graph: None,
            },
            parser::ProgressEvent::EvaluationStarted(path) => {
                Self::phase(ProgressKind::EvaluationStarted, path)
            }
            parser::ProgressEvent::Text(_) => return None,
        })
    }
}
