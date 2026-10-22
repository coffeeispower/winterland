use super::nix_command;
use crate::internal_json::{parser, Progress, ProgressKind};
use crate::{DependencyGraph, Node, OutputError, ShovelConfig};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::sync::mpsc;
use tokio_util::io::SyncIoBridge;

#[cfg(feature = "flutter_rust_bridge")]
use crate::frb_generated::StreamSink;

const METADATA_EXPRESSION: &str = include_str!("metadata-prober.nix");

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

    /// Evaluates this output and reports progress while obtaining the complete
    /// derivation dependency graph that Nix would need to build.
    ///
    /// This only evaluates the flake and inspects its derivations. It does not
    /// build any store paths.
    #[cfg(feature = "flutter_rust_bridge")]
    pub async fn show(&self, sink: StreamSink<Progress>) -> Result<(), OutputError> {
        let graph = self
            .show_with_progress(|event| {
                let _ = sink.add(event);
            })
            .await?;
        let _ = sink.add(Progress {
            kind: ProgressKind::Completed,
            value: String::new(),
            done: None,
            expected: None,
            running: None,
            failed: None,
            graph: Some(graph),
            tree: None,
        });
        Ok(())
    }

    /// Evaluates the derivation graph and reports progress to a Rust callback.
    #[cfg(not(feature = "flutter_rust_bridge"))]
    pub async fn show(
        &self,
        mut on_progress: impl FnMut(Progress),
    ) -> Result<DependencyGraph, OutputError> {
        let graph = self.show_with_progress(&mut on_progress).await?;
        on_progress(Progress {
            kind: ProgressKind::Completed,
            value: String::new(),
            done: None,
            expected: None,
            running: None,
            failed: None,
            graph: None,
            tree: None,
        });
        Ok(graph)
    }

    async fn show_with_progress(
        &self,
        mut report: impl FnMut(Progress),
    ) -> Result<DependencyGraph, OutputError> {
        let flake_url = self.get_flake_url();
        let mut child = nix_command::command()
            .args([
                "derivation",
                "show",
                "--recursive",
                "--log-format",
                "internal-json",
                &flake_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| OutputError::NixFailed {
                status: None,
                stderr: error.to_string(),
            })?;

        let stdout = child.stdout.take().ok_or_else(|| OutputError::NixFailed {
            status: None,
            stderr: "nix stdout was not captured".to_owned(),
        })?;
        let stderr = child.stderr.take().ok_or_else(|| OutputError::NixFailed {
            status: None,
            stderr: "nix stderr was not captured".to_owned(),
        })?;
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        let stderr_task = tokio::spawn(parser::read_stderr(stderr, message_sender));
        let mut graph_task = Box::pin(tokio::task::spawn_blocking(move || {
            DependencyGraph::from_reader(SyncIoBridge::new(stdout))
        }));
        let mut child_wait = Box::pin(child.wait());
        let mut graph_result = None;
        let mut status_result = None;
        let mut stderr_finished = false;
        let mut stderr_output = String::new();

        while graph_result.is_none() || status_result.is_none() || !stderr_finished {
            tokio::select! {
                result = &mut graph_task, if graph_result.is_none() => {
                    graph_result = Some(result);
                }
                result = &mut child_wait, if status_result.is_none() => {
                    status_result = Some(result);
                }
                message = message_receiver.recv(), if !stderr_finished => {
                    match message {
                        Some(message) => parser::process_stderr_message(
                            message,
                            &mut stderr_output,
                            |event| {
                                if let Some(progress) = Progress::from_event(event) {
                                    report(progress);
                                }
                            },
                        ),
                        None => stderr_finished = true,
                    }
                }
            }
        }

        stderr_task.await.map_err(|_| OutputError::NixFailed {
            status: status_result
                .as_ref()
                .and_then(|result| result.as_ref().ok()?.code()),
            stderr: "nix stderr reader task panicked".to_owned(),
        })?;

        let status = status_result
            .expect("the process wait future must complete before the query loop exits")
            .map_err(|error| OutputError::NixFailed {
                status: None,
                stderr: error.to_string(),
            })?;
        let graph_result = graph_result
            .expect("the graph deserialization task must complete before the query loop exits")
            .map_err(|_| OutputError::NixFailed {
                status: status.code(),
                stderr: "nix graph deserialization task panicked".to_owned(),
            })?;
        if !status.success() {
            return Err(OutputError::NixFailed {
                status: status.code(),
                stderr: stderr_output,
            });
        }

        let graph = graph_result.map_err(OutputError::InvalidNixOutput)?;
        enrich_graph_with_metadata(&flake_url, graph, &mut report).await
    }
}

async fn enrich_graph_with_metadata(
    flake_url: &str,
    mut graph: DependencyGraph,
    report: &mut impl FnMut(Progress),
) -> Result<DependencyGraph, OutputError> {
    let mut child = nix_command::command()
        .args([
            "eval",
            "--json",
            "--log-format",
            "internal-json",
            "--apply",
            METADATA_EXPRESSION,
            flake_url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| OutputError::NixFailed {
            status: None,
            stderr: error.to_string(),
        })?;

    let stdout = child.stdout.take().ok_or_else(|| OutputError::NixFailed {
        status: None,
        stderr: "nix metadata stdout was not captured".to_owned(),
    })?;
    let stderr = child.stderr.take().ok_or_else(|| OutputError::NixFailed {
        status: None,
        stderr: "nix metadata stderr was not captured".to_owned(),
    })?;
    let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
    let stderr_task = tokio::spawn(parser::read_stderr(stderr, message_sender));
    let mut metadata_task = Box::pin(tokio::task::spawn_blocking(move || {
        let result = graph.apply_metadata_reader(SyncIoBridge::new(stdout));
        (graph, result)
    }));
    let mut child_wait = Box::pin(child.wait());
    let mut metadata_result = None;
    let mut status_result = None;
    let mut stderr_finished = false;
    let mut stderr_output = String::new();

    while metadata_result.is_none() || status_result.is_none() || !stderr_finished {
        tokio::select! {
            result = &mut metadata_task, if metadata_result.is_none() => {
                metadata_result = Some(result);
            }
            result = &mut child_wait, if status_result.is_none() => {
                status_result = Some(result);
            }
            message = message_receiver.recv(), if !stderr_finished => {
                match message {
                    Some(message) => parser::process_stderr_message(
                        message,
                        &mut stderr_output,
                        |event| {
                            if let Some(progress) = Progress::from_event(event) {
                                report(progress);
                            }
                        },
                    ),
                    None => stderr_finished = true,
                }
            }
        }
    }

    stderr_task.await.map_err(|_| OutputError::NixFailed {
        status: status_result
            .as_ref()
            .and_then(|result| result.as_ref().ok()?.code()),
        stderr: "nix metadata stderr reader task panicked".to_owned(),
    })?;

    let status = status_result
        .expect("the metadata process wait future must complete before the query loop exits")
        .map_err(|error| OutputError::NixFailed {
            status: None,
            stderr: error.to_string(),
        })?;
    let (graph, metadata_result) = metadata_result
        .expect("the metadata deserialization task must complete before the query loop exits")
        .map_err(|_| OutputError::NixFailed {
            status: status.code(),
            stderr: "nix metadata deserialization task panicked".to_owned(),
        })?;
    if !status.success() {
        return Err(OutputError::NixFailed {
            status: status.code(),
            stderr: stderr_output,
        });
    }

    metadata_result.map_err(OutputError::InvalidNixOutput)?;
    Ok(graph)
}
