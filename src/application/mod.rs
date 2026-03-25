use std::io::{BufRead, Write};

use anyhow::{Context, Result};

use crate::domain::Purifier;

/// Drives the purification pipeline over a byte stream.
///
/// The engine reads the input line-by-line, applies each [`Purifier`] in the
/// order they were added, and writes surviving lines to the output.  A
/// trailing newline (`\n`) is appended to each written line.
///
/// # Type parameters
/// * `R` – Anything that implements [`BufRead`] (e.g. `BufReader<File>`,
///   `BufReader<Stdin>`, or a `Cursor<&[u8]>` in tests).
/// * `W` – Anything that implements [`Write`] (e.g. `BufWriter<Stdout>`,
///   `BufWriter<File>`, or a `Vec<u8>` in tests).
pub struct PurificationEngine<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    purifiers: Vec<Box<dyn Purifier>>,
}

impl<R: BufRead, W: Write> PurificationEngine<R, W> {
    /// Create a new engine.
    ///
    /// # Arguments
    /// * `reader`   – Buffered reader supplying the input bytes.
    /// * `writer`   – Writer receiving the purified output.
    /// * `purifiers` – Ordered list of purifiers to apply.
    pub fn new(reader: R, writer: W, purifiers: Vec<Box<dyn Purifier>>) -> Self {
        Self {
            reader,
            writer,
            purifiers,
        }
    }

    /// Run the engine until EOF.
    ///
    /// For each line:
    /// 1. Strip the trailing `\n` (and `\r\n` on Windows).
    /// 2. Pass the raw bytes through every purifier in order.
    /// 3. Write the result if none of the purifiers returned `None`.
    ///
    /// After all lines have been processed, [`Purifier::finalize`] is called
    /// on every purifier.
    ///
    /// # Errors
    /// Returns an error if any I/O operation fails.
    pub fn run(mut self) -> Result<()> {
        let mut line_buf = Vec::new();

        loop {
            line_buf.clear();
            let bytes_read = self
                .reader
                .read_until(b'\n', &mut line_buf)
                .context("failed to read from input")?;

            if bytes_read == 0 {
                break; // EOF
            }

            // Strip trailing newline characters.
            if line_buf.last() == Some(&b'\n') {
                line_buf.pop();
            }
            if line_buf.last() == Some(&b'\r') {
                line_buf.pop();
            }

            // Apply purifiers sequentially; short-circuit on `None`.
            let mut current: Option<Vec<u8>> = Some(line_buf.clone());
            for purifier in &self.purifiers {
                current = current.and_then(|bytes| purifier.purify(&bytes));
                if current.is_none() {
                    break;
                }
            }

            // Write the surviving line with a trailing newline.
            if let Some(bytes) = current {
                self.writer
                    .write_all(&bytes)
                    .context("failed to write output")?;
                self.writer
                    .write_all(b"\n")
                    .context("failed to write newline")?;
            }
        }

        self.writer.flush().context("failed to flush output")?;

        // Give every purifier a chance to emit summary information.
        for purifier in &self.purifiers {
            purifier.finalize();
        }

        Ok(())
    }
}
