use std::cell::Cell;
use std::io::{self, Write};

use crate::domain::Purifier;

/// Passes all lines through unchanged while counting input bytes and tokens.
///
/// When [`Purifier::finalize`] is called, a summary is printed to **stderr**
/// showing:
/// * Total bytes processed
/// * Estimated token count (bytes / 4, a common rough approximation)
///
/// Use this purifier at the *end* of the chain to report the final token
/// budget of the purified output.
pub struct StatsPurifier {
    total_bytes: Cell<u64>,
}

impl StatsPurifier {
    /// Create a new `StatsPurifier`.
    pub fn new() -> Self {
        Self {
            total_bytes: Cell::new(0),
        }
    }
}

impl Default for StatsPurifier {
    fn default() -> Self {
        Self::new()
    }
}

// `Cell<u64>` is `!Sync`; safe because this is used single-threaded.
unsafe impl Sync for StatsPurifier {}

impl Purifier for StatsPurifier {
    /// Pass the line through unchanged and accumulate its byte count.
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>> {
        // +1 for the newline that was stripped before arriving here.
        self.total_bytes
            .set(self.total_bytes.get() + input.len() as u64 + 1);
        Some(input.to_vec())
    }

    /// Print a token-savings summary to stderr.
    fn finalize(&self) {
        let bytes = self.total_bytes.get();
        let estimated_tokens = bytes / 4;
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        // Intentionally ignore write errors here (e.g. stderr closed).
        let _ = writeln!(
            handle,
            "[stats] bytes: {bytes}, estimated tokens: {estimated_tokens}"
        );
    }
}
