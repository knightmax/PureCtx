use std::io::{self, BufReader, BufWriter};

use anyhow::Result;

use crate::application::PurificationEngine;
use crate::domain::Purifier;

/// Build and run the purification engine over stdin → stdout.
///
/// The `purifiers` list is consumed and forwarded to a [`PurificationEngine`]
/// backed by the real stdin and stdout.
///
/// # SIGPIPE handling
/// `BrokenPipe` errors are silently ignored so that `pure | head -5` behaves
/// correctly (the process exits cleanly when the consumer closes the pipe).
///
/// # Errors
/// Returns any non-BrokenPipe I/O error wrapped in [`anyhow::Error`].
pub fn run_stdio(purifiers: Vec<Box<dyn Purifier>>) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();

    let reader = BufReader::new(stdin.lock());
    let writer = BufWriter::new(stdout.lock());

    let engine = PurificationEngine::new(reader, writer, purifiers);
    match engine.run() {
        Ok(()) => Ok(()),
        Err(e) => {
            // Ignore BrokenPipe – this happens when downstream consumers
            // (e.g. `head`) close the pipe before we finish writing.
            if is_broken_pipe(&e) { Ok(()) } else { Err(e) }
        }
    }
}

/// Return `true` when the error chain contains a `BrokenPipe` I/O error.
fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        e.downcast_ref::<io::Error>()
            .map(|io| io.kind() == io::ErrorKind::BrokenPipe)
            .unwrap_or(false)
    })
}
