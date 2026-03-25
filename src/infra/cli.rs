use clap::{Parser, Subcommand};

/// `pure` – PureCtx context purification utility for LLMs.
///
/// Reads from stdin and writes purified output to stdout.  Various
/// sub-commands transform the stream in composable ways; pipe them together
/// for multi-step processing:
///
/// ```text
/// cat file.rs | pure sift --exclude "^#" | pure clean | pure stats
/// ```
#[derive(Debug, Parser)]
#[command(
    name = "pure",
    version,
    about = "Purify LLM context: filter, extract, clean and measure",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level sub-commands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Filter lines using regular expressions (include / exclude).
    Sift(SiftArgs),
    /// Extract structural blocks between start and end patterns.
    Snip(SnipArgs),
    /// Remove comments, blank lines, and excess indentation.
    Clean(CleanArgs),
    /// Report byte / token statistics to stderr (passes data through).
    Stats,
}

/// Arguments for the `sift` sub-command.
#[derive(Debug, clap::Args)]
pub struct SiftArgs {
    /// Keep only lines matching this regex.
    #[arg(long, value_name = "REGEX")]
    pub include: Option<String>,

    /// Drop lines matching this regex.
    #[arg(long, value_name = "REGEX")]
    pub exclude: Option<String>,
}

/// Arguments for the `snip` sub-command.
#[derive(Debug, clap::Args)]
pub struct SnipArgs {
    /// Regex that marks the beginning of a block.
    #[arg(long, value_name = "PATTERN")]
    pub start: String,

    /// Regex that marks the end of a block.
    #[arg(long, value_name = "PATTERN")]
    pub end: String,

    /// Include the delimiter lines themselves in the output.
    #[arg(long, default_value_t = false)]
    pub inclusive: bool,
}

/// Arguments for the `clean` sub-command.
#[derive(Debug, clap::Args)]
pub struct CleanArgs {
    /// Do not strip single-line or block comments.
    #[arg(long)]
    pub no_comments: bool,

    /// Do not remove blank lines.
    #[arg(long)]
    pub no_empty_lines: bool,

    /// Do not collapse leading indentation to a single space.
    #[arg(long)]
    pub no_minify_indent: bool,
}
