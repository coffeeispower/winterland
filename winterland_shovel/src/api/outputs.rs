#[cfg(feature = "flutter_rust_bridge")]
use crate::frb_generated::StreamSink;
use crate::internal_json::{parser, types::Message as NixMessage};
use crate::ShovelConfig;
use bumpalo::Bump;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{self, UnboundedSender};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// Describes the value represented by an output-tree node.
///
/// Container kinds describe nodes that lead to more output-path segments or
/// metadata. Scalar kinds describe values attached to those nodes. A
/// [`Kind::Derivation`] node identifies an output that can be handed to a
/// [`crate::Builder`] for building.
pub enum Kind {
    Object,
    Derivation,
    Array,
    Null,
    Boolean,
    Integer,
    Float,
    NumberString,
    String,
}

/// The phase of a query progress event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb())]
pub enum QueryProgressKind {
    /// Nix started downloading or copying a flake input.
    InputDownloadStarted,
    /// Nix finished downloading or copying a flake input.
    InputDownloadFinished,
    /// Nix reported byte-level progress for an input download.
    InputDownloadProgress,
    /// Nix started evaluating an output path in the flake.
    EvaluationStarted,
    /// Nix finished querying the flake and the event contains the tree in the
    /// Flutter bridge API.
    Completed,
}

/// One progress event emitted while querying a flake.
///
/// Download descriptions are taken from Nix's internal JSON log messages and
/// may vary between Nix versions. Native Rust callers receive the completed
/// tree as the return value of `Tree::query`; Flutter callers receive it in
/// the `tree` field of the final `completed` event because FRB represents a
/// `StreamSink` function as a Dart stream.
#[derive(Debug)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb())]
pub struct QueryProgress {
    /// The phase transition represented by this event.
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
    /// The completed tree for [`QueryProgressKind::Completed`].
    pub tree: Option<Tree>,
}

impl QueryProgress {
    fn phase(kind: QueryProgressKind, value: String) -> Self {
        Self {
            kind,
            value,
            done: None,
            expected: None,
            running: None,
            failed: None,
            tree: None,
        }
    }
}

/// The minimal representation needed while converting `nix flake show` JSON.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NixValue {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<NixValue>),
    Object(BTreeMap<String, NixValue>),
}

/// One segment of an output path in a [`Tree`].
///
/// For an output such as `packages.x86_64-linux.libdrm`, the path is represented
/// by the `packages`, `x86_64-linux`, and `libdrm` nodes. A node's children are
/// the next path segments that can be selected from it; for example,
/// `packages` may have `x86_64-linux` and `aarch64-linux` children.
///
/// Derivation nodes are selectable output leaves and can be passed to a
/// [`crate::Builder`]. The node and all of its links are allocated in the
/// owning tree's bump arena. Parent and child links use direct pointers rather
/// than IDs into a separate node table.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
pub struct Node<'arena> {
    pub name: &'arena str,
    pub kind: Kind,
    pub value: Option<&'arena str>,
    pub children: &'arena [*const Node<'arena>],
    pub parent: Option<*const Node<'arena>>,
}

impl<'arena> Node<'arena> {
    /// Equivalent to Zig's `Node.buildNodeName`.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn build_node_name(&self) -> String {
        let mut names = Vec::new();
        let mut current = Some(self);
        while let Some(node) = current {
            names.push(node.name);
            current = node.parent.map(|parent| unsafe { &*parent });
        }
        names.reverse();
        names.join(".")
    }
}

/// A snapshot of the outputs declared by a Nix flake.
///
/// The tree is created by [`Tree::query`] from `nix flake show --json
/// --all-systems`. It owns the bump arena containing the root, every output
/// path node, every child slice, and every string referenced by those nodes.
/// Traversal through [`Tree::root`] and [`Tree::find`] is immutable.
pub struct Tree {
    #[allow(dead_code)]
    arena: Bump,
    root: *const Node<'static>,
}

// Tree is immutable after construction. Its raw pointers refer only to nodes
// and strings owned by its private arena, so sharing the complete tree between
// bridge worker threads is safe.
unsafe impl Send for Tree {}
unsafe impl Sync for Tree {}

unsafe impl<'arena> Send for Node<'arena> {}
unsafe impl<'arena> Sync for Node<'arena> {}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree").finish_non_exhaustive()
    }
}

/// Errors that can occur while querying or selecting flake outputs.
#[derive(Debug)]
pub enum OutputError {
    NixFailed { status: Option<i32>, stderr: String },
    InvalidNixOutput(serde_json::Error),
    OutputNotFound(String),
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NixFailed { status, stderr } => {
                write!(f, "nix flake show failed ({status:?}): {stderr}")
            }
            Self::InvalidNixOutput(error) => write!(f, "invalid nix JSON output: {error}"),
            Self::OutputNotFound(path) => write!(f, "output path not found: {path}"),
        }
    }
}

impl std::error::Error for OutputError {}

impl Tree {
    /// Queries Nix and builds an immutable snapshot of the flake's outputs.
    ///
    /// The snapshot includes all systems reported by `nix flake show`, so a
    /// caller can traverse paths such as `packages.x86_64-linux.libdrm`.
    #[cfg(feature = "flutter_rust_bridge")]
    pub async fn query(
        config: &ShovelConfig,
        sink: StreamSink<QueryProgress>,
    ) -> Result<(), OutputError> {
        let tree = Self::query_with_progress(config, |event| {
            let _ = sink.add(event);
        })
        .await?;
        let _ = sink.add(QueryProgress {
            kind: QueryProgressKind::Completed,
            value: String::new(),
            done: None,
            expected: None,
            running: None,
            failed: None,
            tree: Some(tree),
        });
        // FRB exposes StreamSink functions as Dart streams. The completed tree
        // is therefore delivered by the final event above.
        Ok(())
    }

    /// Queries Nix and reports progress to a Rust callback when the Flutter
    /// bridge is not enabled.
    #[cfg(not(feature = "flutter_rust_bridge"))]
    pub async fn query(
        config: &ShovelConfig,
        mut on_progress: impl FnMut(QueryProgress),
    ) -> Result<Self, OutputError> {
        let tree = Self::query_with_progress(config, &mut on_progress).await?;
        on_progress(QueryProgress {
            kind: QueryProgressKind::Completed,
            value: String::new(),
            done: None,
            expected: None,
            running: None,
            failed: None,
            tree: None,
        });
        Ok(tree)
    }

    async fn query_with_progress(
        config: &ShovelConfig,
        mut report: impl FnMut(QueryProgress),
    ) -> Result<Self, OutputError> {
        let flake = format!("path:{}", config.flake_path());
        let mut child = Command::new("nix")
            .args([
                "flake",
                "show",
                "--json",
                "--all-systems",
                "--log-format",
                "internal-json",
                &flake,
            ])
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
        let stderr = child.stderr.take().ok_or_else(|| OutputError::NixFailed {
            status: None,
            stderr: "nix stderr was not captured".to_owned(),
        })?;
        let (message_sender, mut message_receiver) = mpsc::unbounded_channel();
        let stderr_task = tokio::spawn(read_stderr(stderr, message_sender));
        let mut stdout_output = Vec::new();
        let mut stdout_read = Box::pin(stdout.read_to_end(&mut stdout_output));
        let mut child_wait = Box::pin(child.wait());
        let mut stdout_result = None;
        let mut status_result = None;
        let mut stderr_finished = false;

        let mut stderr_output = String::new();
        while stdout_result.is_none() || status_result.is_none() || !stderr_finished {
            tokio::select! {
                result = &mut stdout_read, if stdout_result.is_none() => {
                    stdout_result = Some(result);
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
        let stdout_error = stdout_result
            .expect("the stdout read future must complete before the query loop exits");
        if let Err(error) = stdout_error {
            return Err(OutputError::NixFailed {
                status: status.code(),
                stderr: error.to_string(),
            });
        }
        if !status.success() {
            return Err(OutputError::NixFailed {
                status: status.code(),
                stderr: stderr_output,
            });
        }

        let value: NixValue =
            serde_json::from_slice(&stdout_output).map_err(OutputError::InvalidNixOutput)?;
        Ok(Self::from_value(&value))
    }

    fn from_value(value: &NixValue) -> Self {
        let arena = Bump::new();
        let root = build_node(&arena, None, "outputs", value);
        let root: *const Node<'_> = root;
        let root: *const Node<'static> = root as *const Node<'static>;
        Self { arena, root }
    }

    /// Returns the root node representing the complete set of flake outputs.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn root(&self) -> &Node<'_> {
        // SAFETY: `root` points to a node allocated in `arena` during tree
        // construction, and `&self` guarantees that the arena is still alive.
        unsafe { &*self.root }
    }

    /// Finds the node reached by following each output-path segment in order.
    ///
    /// An empty path returns the root node. The returned node is borrowed from
    /// this tree and cannot outlive it.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(ignore))]
    pub fn find(&self, path: &[&str]) -> Option<&Node<'_>> {
        path.iter().try_fold(self.root(), |node, segment| {
            node.children.iter().find_map(|child| unsafe {
                let child = &**child;
                (child.name == *segment).then_some(child)
            })
        })
    }

    /// Serializes this output snapshot into a JSON representation of the tree.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        #[derive(Serialize)]
        struct JsonNode<'a> {
            name: &'a str,
            kind: Kind,
            value: Option<&'a str>,
            children: Vec<JsonNode<'a>>,
        }

        fn convert<'a>(node: &'a Node<'a>) -> JsonNode<'a> {
            JsonNode {
                name: node.name,
                kind: node.kind,
                value: node.value,
                children: node
                    .children
                    .iter()
                    .map(|child| unsafe { convert(&**child) })
                    .collect(),
            }
        }

        serde_json::to_string(&convert(self.root()))
    }

    /// Prints the output snapshot in a tree-shaped format for native debugging.
    #[cfg_attr(feature = "flutter_rust_bridge", flutter_rust_bridge::frb(sync))]
    pub fn debug_print_outputs(&self) {
        fn print_children(children: &[*const Node<'_>], prefix: &mut String) {
            for (index, child) in children.iter().enumerate() {
                let child = unsafe { &**child };
                let is_last = index + 1 == children.len();
                let branch = if is_last { "└── " } else { "├── " };
                match child.value {
                    Some(value) => println!(
                        "{}{branch}{} [{:?}]: {value}",
                        prefix, child.name, child.kind
                    ),
                    None => println!("{}{branch}{} [{:?}]", prefix, child.name, child.kind),
                }
                let previous_len = prefix.len();
                prefix.push_str(if is_last { "    " } else { "│   " });
                print_children(child.children, prefix);
                prefix.truncate(previous_len);
            }
        }

        let root = self.root();
        println!("{} [{:?}]", root.name, root.kind);
        print_children(root.children, &mut String::new());
    }
}

enum StderrMessage {
    Parsed(NixMessage),
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

fn process_stderr_message(
    message: StderrMessage,
    report: &mut impl FnMut(QueryProgress),
    stderr_output: &mut String,
) {
    match message {
        StderrMessage::Parsed(message) => match message {
            NixMessage::InputDownloadStarted { description, .. } => {
                report(QueryProgress::phase(
                    QueryProgressKind::InputDownloadStarted,
                    description,
                ));
            }
            NixMessage::InputDownloadFinished { description, .. } => {
                report(QueryProgress::phase(
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
            } => report(QueryProgress {
                kind: QueryProgressKind::InputDownloadProgress,
                value: description,
                done: Some(done),
                expected: Some(expected),
                running: Some(running),
                failed: Some(failed),
                tree: None,
            }),
            NixMessage::DownloadExpected {
                description,
                expected,
                ..
            } => report(QueryProgress {
                kind: QueryProgressKind::InputDownloadProgress,
                value: description,
                done: None,
                expected: Some(expected),
                running: None,
                failed: None,
                tree: None,
            }),
            NixMessage::EvaluationStarted { path, .. } => {
                report(QueryProgress::phase(
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

fn build_node<'arena>(
    arena: &'arena Bump,
    parent: Option<*const Node<'arena>>,
    name: &str,
    value: &NixValue,
) -> &'arena Node<'arena> {
    let (kind, scalar) = match value {
        NixValue::Object(object) => (
            if object.contains_key("derivation")
                || matches!(object.get("type"), Some(NixValue::String(value)) if value == "derivation")
            {
                Kind::Derivation
            } else {
                Kind::Object
            },
            None,
        ),
        NixValue::Array(_) => (Kind::Array, None),
        NixValue::Null => (Kind::Null, Some("null".to_owned())),
        NixValue::Bool(value) => (Kind::Boolean, Some(value.to_string())),
        NixValue::Integer(value) => (Kind::Integer, Some(value.to_string())),
        NixValue::Float(value) => (Kind::Float, Some(value.to_string())),
        NixValue::String(value) => (Kind::String, Some(value.clone())),
    };

    let name = arena.alloc_str(name);
    let scalar_value = scalar.map(|value| &*arena.alloc_str(&value));
    let node_ptr: *mut Node<'arena> = arena.alloc(Node {
        name,
        kind,
        value: scalar_value,
        children: &[],
        parent,
    });
    let node_ref = node_ptr as *const Node<'arena>;

    let child_capacity = match value {
        NixValue::Object(object) => object.len(),
        NixValue::Array(values) => values.len(),
        _ => 0,
    };
    let mut children: bumpalo::collections::Vec<'arena, *const Node<'arena>> =
        bumpalo::collections::Vec::with_capacity_in(child_capacity, arena);
    match value {
        NixValue::Object(object) => {
            for (name, value) in object {
                children
                    .push(build_node(arena, Some(node_ref), name, value) as *const Node<'arena>);
            }
        }
        NixValue::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                children.push(
                    build_node(arena, Some(node_ref), &format!("[{index}]"), value)
                        as *const Node<'arena>,
                );
            }
        }
        _ => {}
    }
    let children: &'arena [*const Node<'arena>] = children.into_bump_slice();
    unsafe {
        (*node_ptr).children = children;
        &*node_ptr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_node_name_from_parent_links() {
        let value: NixValue = serde_json::from_str(
            "{\"packages\":{\"x86_64-linux\":{\"hello\":{\"derivation\":{\"name\":\"hello\"}}}}}",
        )
        .unwrap();
        let tree = Tree::from_value(&value);
        let node = tree.find(&["packages", "x86_64-linux", "hello"]).unwrap();
        assert_eq!(
            node.build_node_name(),
            "outputs.packages.x86_64-linux.hello"
        );
        let parent = node.parent.unwrap();
        assert_eq!(unsafe { (*parent).name }, "x86_64-linux");
    }
}
