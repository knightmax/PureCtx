use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when loading or validating a filter file.
#[derive(Debug, Error)]
pub enum FilterError {
    /// The TOML content could not be parsed.
    #[error("failed to parse filter TOML: {0}")]
    ParseError(#[from] toml::de::Error),
    /// A regex pattern inside a pipeline rule is invalid.
    #[error("invalid regex in rule: {0}")]
    InvalidPattern(#[from] regex::Error),
    /// The filter file has no pipeline rules.
    #[error("filter `{0}` has an empty pipeline")]
    EmptyPipeline(String),
}

/// A complete filter definition loaded from a TOML file.
///
/// # Example TOML
///
/// ```toml
/// name = "maven"
/// version = 1
/// description = "Reduces Maven build noise for LLM context"
///
/// [match]
/// command = "mvn"
/// aliases = ["mvnw", "./mvnw"]
///
/// [[pipeline]]
/// action = "remove_lines"
/// pattern = "^\\[INFO\\] Download"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterFile {
    /// Unique filter name.
    pub name: String,
    /// Filter format version (for forward compatibility).
    #[serde(default = "default_version")]
    pub version: u32,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Matching rules that determine when this filter activates.
    #[serde(rename = "match")]
    pub match_rules: MatchRules,
    /// Ordered list of pipeline actions to apply.
    pub pipeline: Vec<PipelineAction>,
    /// Error handling strategy.
    #[serde(default)]
    pub on_error: OnError,
}

fn default_version() -> u32 {
    1
}

/// Rules that determine whether this filter should be applied to a given
/// command invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRules {
    /// The base command name (e.g. `"mvn"`, `"npm"`, `"dotnet"`).
    pub command: String,
    /// Optional alternative command names (e.g. `["mvnw", "./mvnw"]`).
    #[serde(default)]
    pub aliases: Vec<String>,
    /// If present, the first argument must match one of these (e.g. `"test"`).
    #[serde(default)]
    pub subcommand: Option<String>,
}

/// A single step in the filter pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PipelineAction {
    /// Remove lines matching a regex pattern.
    RemoveLines {
        /// The regex pattern to match against.
        pattern: String,
    },
    /// Keep only lines matching a regex pattern.
    KeepLines {
        /// The regex pattern to match against.
        pattern: String,
    },
    /// Remove ANSI escape sequences (colors, formatting).
    StripAnsi,
    /// Remove blank lines (lines containing only whitespace).
    RemoveEmptyLines,
    /// Keep only the first N lines.
    Head {
        /// Maximum number of lines to keep.
        count: usize,
    },
    /// Keep only the last N lines.
    Tail {
        /// Maximum number of lines to keep.
        count: usize,
    },
}

/// What to do when a filter encounters an error.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    /// Output the raw command result (default).
    #[default]
    Passthrough,
    /// Propagate the error.
    Fail,
}

impl FilterFile {
    /// Parse a filter from a TOML string.
    ///
    /// # Errors
    /// Returns [`FilterError`] if the TOML is invalid or contains bad regex patterns.
    pub fn from_toml(content: &str) -> Result<Self, FilterError> {
        let filter: Self = toml::from_str(content)?;
        if filter.pipeline.is_empty() {
            return Err(FilterError::EmptyPipeline(filter.name));
        }
        // Validate all regex patterns at load time.
        for action in &filter.pipeline {
            match action {
                PipelineAction::RemoveLines { pattern } | PipelineAction::KeepLines { pattern } => {
                    regex::bytes::Regex::new(pattern)?;
                }
                _ => {}
            }
        }
        Ok(filter)
    }

    /// Check whether this filter matches the given command and arguments.
    pub fn matches(&self, command: &str, _args: &[String]) -> bool {
        let cmd_base = command.rsplit('/').next().unwrap_or(command);

        let matches_cmd = cmd_base == self.match_rules.command
            || self
                .match_rules
                .aliases
                .iter()
                .any(|a| a.rsplit('/').next().unwrap_or(a) == cmd_base);

        if !matches_cmd {
            return false;
        }

        if let Some(ref sub) = self.match_rules.subcommand {
            return _args.first().map(|a| a.as_str()) == Some(sub.as_str());
        }

        true
    }
}
