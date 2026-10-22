use tokio::process::Command;

/// Creates a Nix CLI command with the features required by Winterland.
pub(crate) fn command() -> Command {
    let mut command = Command::new("nix");
    command.args([
        "--extra-experimental-features",
        "nix-command flakes pipe-operators",
    ]);
    command
}
