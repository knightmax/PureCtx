/// Domain layer: core abstractions and purifier implementations.
pub mod clean;
pub mod filter;
pub mod sift;
pub mod snip;
pub mod stats;
pub mod tracking;

/// A `Purifier` processes a single line of bytes and decides whether to keep
/// it (possibly transformed) or discard it.
///
/// Returning `Some(bytes)` keeps the line (with the provided content).
/// Returning `None` discards the line entirely.
///
/// Implementations that need to track state across lines (e.g. `SnipPurifier`
/// or `CleanPurifier` for multi-line comments) must use interior mutability.
pub trait Purifier: Send + Sync {
    /// Process a single line.
    ///
    /// # Arguments
    /// * `input` – Raw bytes of the line **without** the trailing newline.
    ///
    /// # Returns
    /// * `Some(Vec<u8>)` – The (possibly transformed) line to emit.
    /// * `None` – The line should be dropped.
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>>;

    /// Called once after all lines have been processed.
    ///
    /// Default implementation is a no-op.  Override to emit summary
    /// information (e.g. token statistics to stderr).
    fn finalize(&self) {}
}
