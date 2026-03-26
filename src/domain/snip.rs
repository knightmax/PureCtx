use std::cell::Cell;

use regex::bytes::Regex;
use thiserror::Error;

use crate::domain::Purifier;

/// State machine for the [`SnipPurifier`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum SnipState {
    /// We are outside any block; searching for the start pattern.
    Outside,
    /// We are inside a block; searching for the end pattern.
    Inside,
}

/// Errors that can occur when constructing a [`SnipPurifier`].
#[derive(Debug, Error)]
pub enum SnipError {
    /// A provided regex pattern is invalid.
    #[error("invalid regex pattern `{pattern}`: {source}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: regex::Error,
    },
}

/// Extracts structural blocks delimited by start and end regex patterns.
///
/// Lines between the start and end markers are kept; everything else is
/// discarded.  Multiple blocks can appear in the input; each will be
/// extracted in order.
///
/// Interior mutability (`Cell`) is used to track state across lines because
/// the [`Purifier`] trait requires `&self`.
pub struct SnipPurifier {
    start: Regex,
    end: Regex,
    /// When `true`, the delimiter lines themselves are included in the output.
    inclusive: bool,
    state: Cell<SnipState>,
}

impl SnipPurifier {
    /// Build a new `SnipPurifier`.
    ///
    /// # Arguments
    /// * `start`     – Regex pattern that marks the beginning of a block.
    /// * `end`       – Regex pattern that marks the end of a block.
    /// * `inclusive` – When `true`, the delimiter lines are kept in the output.
    ///
    /// # Errors
    /// Returns [`SnipError::InvalidPattern`] if either pattern is invalid.
    pub fn new(start: &str, end: &str, inclusive: bool) -> Result<Self, SnipError> {
        let start = Regex::new(start).map_err(|e| SnipError::InvalidPattern {
            pattern: start.to_owned(),
            source: e,
        })?;
        let end = Regex::new(end).map_err(|e| SnipError::InvalidPattern {
            pattern: end.to_owned(),
            source: e,
        })?;
        Ok(Self {
            start,
            end,
            inclusive,
            state: Cell::new(SnipState::Outside),
        })
    }
}

// `Cell<SnipState>` is `!Sync` by default; we declare it safe here because
// `SnipPurifier` is always driven from a single thread.
unsafe impl Sync for SnipPurifier {}

impl Purifier for SnipPurifier {
    /// Return the line when inside an active block, `None` otherwise.
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>> {
        match self.state.get() {
            SnipState::Outside => {
                if self.start.is_match(input) {
                    self.state.set(SnipState::Inside);
                    if self.inclusive {
                        Some(input.to_vec())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            SnipState::Inside => {
                if self.end.is_match(input) {
                    self.state.set(SnipState::Outside);
                    if self.inclusive {
                        Some(input.to_vec())
                    } else {
                        None
                    }
                } else {
                    Some(input.to_vec())
                }
            }
        }
    }
}
