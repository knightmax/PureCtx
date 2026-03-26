use regex::bytes::Regex;
use thiserror::Error;

use crate::domain::Purifier;

/// Errors that can occur when constructing a [`SiftPurifier`].
#[derive(Debug, Error)]
pub enum SiftError {
    /// A provided regex pattern is invalid.
    #[error("invalid regex pattern `{pattern}`: {source}")]
    InvalidPattern {
        pattern: String,
        #[source]
        source: regex::Error,
    },
}

/// Filters lines using optional include / exclude regular expressions.
///
/// * If an **include** pattern is set, only lines that match it are kept.
/// * If an **exclude** pattern is set, lines that match it are dropped.
/// * Both patterns can be active simultaneously; a line must satisfy
///   *both* constraints to be kept.
pub struct SiftPurifier {
    include: Option<Regex>,
    exclude: Option<Regex>,
}

impl SiftPurifier {
    /// Build a new `SiftPurifier`.
    ///
    /// # Arguments
    /// * `include` – Optional regex; only matching lines are kept.
    /// * `exclude` – Optional regex; matching lines are dropped.
    ///
    /// # Errors
    /// Returns [`SiftError::InvalidPattern`] if either pattern is invalid.
    pub fn new(include: Option<&str>, exclude: Option<&str>) -> Result<Self, SiftError> {
        let include = include
            .map(|p| {
                Regex::new(p).map_err(|e| SiftError::InvalidPattern {
                    pattern: p.to_owned(),
                    source: e,
                })
            })
            .transpose()?;

        let exclude = exclude
            .map(|p| {
                Regex::new(p).map_err(|e| SiftError::InvalidPattern {
                    pattern: p.to_owned(),
                    source: e,
                })
            })
            .transpose()?;

        Ok(Self { include, exclude })
    }
}

impl Purifier for SiftPurifier {
    /// Keep the line only when it satisfies both the include and exclude rules.
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>> {
        if let Some(ref inc) = self.include
            && !inc.is_match(input)
        {
            return None;
        }
        if let Some(ref exc) = self.exclude
            && exc.is_match(input)
        {
            return None;
        }
        Some(input.to_vec())
    }
}
