use crate::internal_json::{parser, types::Message as NixMessage};
use crate::{DependencyGraph, Node, OutputError, QueryProgressKind, ShovelConfig};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_util::io::SyncIoBridge;

#[cfg(feature = "flutter_rust_bridge")]
use crate::frb_generated::StreamSink;

/// Progress emitted while Nix evaluates a derivation build plan.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb())]
pub struct BuilderProgress {
    /// The phase represented by this event.
    pub kind: QueryProgressKind,
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
    /// The evaluated derivation graph for the completed event.
    pub graph: Option<DependencyGraph>,
}

impl BuilderProgress {
    fn phase(kind: QueryProgressKind, value: String) -> Self {
        Self {
            kind,
            value,
            done: None,
            expected: None,
            running: None,
            failed: None,
            graph: None,
        }
    }
}

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
    pub async fn show(&self, sink: StreamSink<BuilderProgress>) -> Result<(), OutputError> {
        let graph = self
            .show_with_progress(|event| {
                let _ = sink.add(event);
            })
            .await?;
        let _ = sink.add(BuilderProgress {
            kind: QueryProgressKind::Completed,
            value: String::new(),
            done: None,
            expected: None,
            running: None,
            failed: None,
            graph: Some(graph),
        });
        Ok(())
    }

    /// Evaluates the derivation graph and reports progress to a Rust callback.
    #[cfg(not(feature = "flutter_rust_bridge"))]
    pub async fn show(
        &self,
        mut on_progress: impl FnMut(BuilderProgress),
    ) -> Result<DependencyGraph, OutputError> {
        let graph = self.show_with_progress(&mut on_progress).await?;
        on_progress(BuilderProgress {
            kind: QueryProgressKind::Completed,
            value: String::new(),
            done: None,
            expected: None,
            running: None,
            failed: None,
            graph: None,
        });
        Ok(graph)
    }

    async fn show_with_progress(
        &self,
        mut report: impl FnMut(BuilderProgress),
    ) -> Result<DependencyGraph, OutputError> {
        let flake_url = self.get_flake_url();
        let mut child = Command::new("nix")
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
        let stderr_task = tokio::spawn(read_stderr(stderr, message_sender));
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
                        Some(message) => process_stderr_message(message, &mut report, &mut stderr_output),
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

        graph_result.map_err(OutputError::InvalidNixOutput)
    }
}

async fn read_stderr<R>(stderr: R, sender: UnboundedSender<StderrMessage>)
where
    R: AsyncRead + Unpin,
{
    let mut processor = parser::Processor::new();
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let message = match processor.process_line(&line) {
            Ok(message) => message,
            Err(_) => NixMessage::Text(line),
        };
        if sender.send(StderrMessage::Parsed(message)).is_err() {
            break;
        }
    }
}

enum StderrMessage {
    Parsed(NixMessage),
}

fn process_stderr_message(
    message: StderrMessage,
    report: &mut impl FnMut(BuilderProgress),
    stderr_output: &mut String,
) {
    match message {
        StderrMessage::Parsed(message) => match message {
            NixMessage::InputDownloadStarted { description, .. } => {
                report(BuilderProgress::phase(
                    QueryProgressKind::InputDownloadStarted,
                    description,
                ));
            }
            NixMessage::InputDownloadFinished { description, .. } => {
                report(BuilderProgress::phase(
                    QueryProgressKind::InputDownloadFinished,
                    description,
                ));
            }
            NixMessage::DownloadProgress {
                description,
                done,
                expected,
                running,
                failed,
                ..
            } => report(BuilderProgress {
                kind: QueryProgressKind::InputDownloadProgress,
                value: description,
                done: Some(done),
                expected: Some(expected),
                running: Some(running),
                failed: Some(failed),
                graph: None,
            }),
            NixMessage::DownloadExpected {
                description,
                expected,
                ..
            } => report(BuilderProgress {
                kind: QueryProgressKind::InputDownloadProgress,
                value: description,
                done: None,
                expected: Some(expected),
                running: None,
                failed: None,
                graph: None,
            }),
            NixMessage::EvaluationStarted { path, .. } => {
                report(BuilderProgress::phase(
                    QueryProgressKind::EvaluationStarted,
                    path,
                ));
            }
            NixMessage::Text(text) => {
                if !text.is_empty() {
                    stderr_output.push_str(&text);
                    stderr_output.push('\n');
                }
            }
            NixMessage::ActivityStarted { .. }
            | NixMessage::ActivityStopped { .. }
            | NixMessage::ActivityResult { .. } => {}
        },
    }
}
