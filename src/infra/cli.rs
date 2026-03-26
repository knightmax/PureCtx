use clap::{Parser, Subcommand};

/// `pure` – PureCtx command-output purification proxy for LLMs.
///
/// Wraps any command and filters its output to reduce noise before feeding it
/// into an LLM. Built-in filters activate automatically for known tools
/// (mvn, npm, cargo, dotnet, gradle). Custom filters can be added via
/// `pure filter add <file>`.
///
/// ```text
/// pure mvn clean install
/// pure npm run build
/// pure cargo test
/// ```
#[derive(Debug, Parser)]
#[command(
    name = "pure",
    version,
    about = "Purify command output for LLM context",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// The command to proxy and filter (everything after `pure`).
    ///
    /// When no subcommand is recognized, the remaining arguments are treated
    /// as a command to execute.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub external: Vec<String>,
}

/// Management sub-commands (filter add / list / show).
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Manage filters (add, list, show).
    Filter(FilterCommand),
}

/// Sub-commands under `pure filter`.
#[derive(Debug, clap::Args)]
pub struct FilterCommand {
    #[command(subcommand)]
    pub action: FilterAction,
}

/// Actions available for `pure filter`.
#[derive(Debug, Subcommand)]
pub enum FilterAction {
    /// Install a custom filter from a TOML file.
    Add(FilterAddArgs),
    /// List all available filters (built-in + custom).
    List,
    /// Show the contents of a named filter.
    Show(FilterShowArgs),
}

/// Arguments for `pure filter add`.
#[derive(Debug, clap::Args)]
pub struct FilterAddArgs {
    /// Path to the filter TOML file to install.
    pub file: String,
}

/// Arguments for `pure filter show`.
#[derive(Debug, clap::Args)]
pub struct FilterShowArgs {
    /// Name of the filter to display.
    pub name: String,
}
