use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use regex::bytes::Regex;

use crate::domain::filter::{FilterFile, PipelineAction};

/// Result of running a proxied command, including filtering statistics.
pub struct ProxyResult {
    /// Exit code of the child process.
    pub exit_code: i32,
    /// Total input bytes (stdout + stderr before filtering).
    pub input_bytes: u64,
    /// Total output bytes (stdout + stderr after filtering).
    pub output_bytes: u64,
}

/// Execute a command, capture its combined stdout+stderr, apply the matched
/// filter pipeline, and write the purified output to the real stdout.
///
/// # Arguments
/// * `command`  – The program to run.
/// * `args`     – Arguments to pass to the program.
/// * `filter`   – An optional filter to apply to the output.
///
/// # Returns
/// A [`ProxyResult`] containing exit code and byte statistics.
///
/// # Errors
/// Returns an error if the child process cannot be spawned or if I/O fails.
pub fn run_proxy(
    command: &str,
    args: &[String],
    filter: Option<&FilterFile>,
) -> Result<ProxyResult> {
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute `{command}`"))?;

    let child_stdout = child.stdout.take().context("failed to capture stdout")?;
    let child_stderr = child.stderr.take().context("failed to capture stderr")?;

    // Merge stdout and stderr into a single stream by reading both.
    // We read stderr in a separate thread to avoid deadlocks.
    let stderr_handle = std::thread::spawn(move || -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut reader = BufReader::new(child_stderr);
        io::copy(&mut reader, &mut buf).context("failed to read child stderr")?;
        Ok(buf)
    });

    let stdout_reader = BufReader::new(child_stdout);

    let mut input_bytes: u64 = 0;
    let mut output_bytes: u64 = 0;

    if let Some(f) = filter {
        let mut counting_writer = CountingWriter::new(BufWriter::new(io::stdout().lock()));
        let in_bytes = process_stream(stdout_reader, &mut counting_writer, &f.pipeline)?;
        input_bytes += in_bytes;
        output_bytes += counting_writer.bytes_written;
    } else {
        let mut counting_writer = CountingWriter::new(BufWriter::new(io::stdout().lock()));
        passthrough(stdout_reader, &mut counting_writer)?;
        input_bytes += counting_writer.bytes_written;
        output_bytes += counting_writer.bytes_written;
    }

    // Now handle stderr
    let stderr_data = stderr_handle
        .join()
        .map_err(|_| anyhow::anyhow!("stderr reader thread panicked"))??;

    if let Some(f) = filter {
        let stderr_reader = BufReader::new(stderr_data.as_slice());
        let mut counting_writer = CountingWriter::new(BufWriter::new(io::stderr().lock()));
        let in_bytes = process_stream(stderr_reader, &mut counting_writer, &f.pipeline)?;
        input_bytes += in_bytes;
        output_bytes += counting_writer.bytes_written;
    } else {
        input_bytes += stderr_data.len() as u64;
        output_bytes += stderr_data.len() as u64;
        let mut stderr_out = io::stderr().lock();
        stderr_out
            .write_all(&stderr_data)
            .context("failed to write stderr")?;
    }

    let status = child.wait().context("failed to wait for child process")?;
    Ok(ProxyResult {
        exit_code: status.code().unwrap_or(1),
        input_bytes,
        output_bytes,
    })
}

/// A writer wrapper that counts bytes written.
struct CountingWriter<W: Write> {
    inner: W,
    bytes_written: u64,
}

impl<W: Write> CountingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            bytes_written: 0,
        }
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.bytes_written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Apply a filter pipeline to a buffered reader, writing results to a writer.
///
/// Returns the number of input bytes read.
fn process_stream<R: BufRead, W: Write>(
    reader: R,
    writer: &mut W,
    pipeline: &[PipelineAction],
) -> Result<u64> {
    // Compile regex patterns once.
    let compiled = compile_pipeline(pipeline)?;

    let mut lines: Vec<Vec<u8>> = Vec::new();
    let mut input_bytes: u64 = 0;
    for line_result in reader.split(b'\n') {
        let line = line_result.context("failed to read line")?;
        input_bytes += line.len() as u64 + 1; // +1 for the newline
        lines.push(line);
    }

    // Apply pipeline actions in order.
    for action in &compiled {
        lines = apply_action(action, lines);
    }

    // Write surviving lines.
    for line in &lines {
        writer.write_all(line).context("failed to write output")?;
        writer.write_all(b"\n").context("failed to write newline")?;
    }
    writer.flush().context("failed to flush output")?;

    Ok(input_bytes)
}

/// Pass data through without any filtering.
fn passthrough<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> Result<()> {
    io::copy(&mut reader, &mut writer).context("failed to copy stream")?;
    writer.flush().context("failed to flush output")?;
    Ok(())
}

/// A compiled pipeline action (regex patterns pre-compiled).
enum CompiledAction {
    RemoveLines(Regex),
    KeepLines(Regex),
    StripAnsi(Regex),
    RemoveEmptyLines,
    Head(usize),
    Tail(usize),
}

/// ANSI escape sequence pattern.
const ANSI_PATTERN: &str = r"\x1B\[[0-9;]*[a-zA-Z]|\x1B\].*?\x07|\x1B\[[\d;]*m";

/// Compile the pipeline actions into their executable form.
fn compile_pipeline(actions: &[PipelineAction]) -> Result<Vec<CompiledAction>> {
    actions
        .iter()
        .map(|a| match a {
            PipelineAction::RemoveLines { pattern } => {
                let re = Regex::new(pattern)
                    .with_context(|| format!("invalid remove_lines pattern: {pattern}"))?;
                Ok(CompiledAction::RemoveLines(re))
            }
            PipelineAction::KeepLines { pattern } => {
                let re = Regex::new(pattern)
                    .with_context(|| format!("invalid keep_lines pattern: {pattern}"))?;
                Ok(CompiledAction::KeepLines(re))
            }
            PipelineAction::StripAnsi => {
                let re = Regex::new(ANSI_PATTERN).context("invalid ANSI pattern")?;
                Ok(CompiledAction::StripAnsi(re))
            }
            PipelineAction::RemoveEmptyLines => Ok(CompiledAction::RemoveEmptyLines),
            PipelineAction::Head { count } => Ok(CompiledAction::Head(*count)),
            PipelineAction::Tail { count } => Ok(CompiledAction::Tail(*count)),
        })
        .collect()
}

/// Apply a single compiled action to a list of lines.
fn apply_action(action: &CompiledAction, lines: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    match action {
        CompiledAction::RemoveLines(re) => lines.into_iter().filter(|l| !re.is_match(l)).collect(),
        CompiledAction::KeepLines(re) => lines.into_iter().filter(|l| re.is_match(l)).collect(),
        CompiledAction::StripAnsi(re) => lines
            .into_iter()
            .map(|l| re.replace_all(&l, &b""[..]).into_owned())
            .collect(),
        CompiledAction::RemoveEmptyLines => lines
            .into_iter()
            .filter(|l| !l.iter().all(|b| b.is_ascii_whitespace()))
            .collect(),
        CompiledAction::Head(n) => lines.into_iter().take(*n).collect(),
        CompiledAction::Tail(n) => {
            let len = lines.len();
            if *n >= len {
                lines
            } else {
                lines.into_iter().skip(len - n).collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_lines() {
        let re = Regex::new("^DEBUG").unwrap();
        let lines = vec![
            b"INFO: hello".to_vec(),
            b"DEBUG: skip".to_vec(),
            b"INFO: world".to_vec(),
        ];
        let result = apply_action(&CompiledAction::RemoveLines(re), lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], b"INFO: hello");
        assert_eq!(result[1], b"INFO: world");
    }

    #[test]
    fn test_keep_lines() {
        let re = Regex::new("ERROR").unwrap();
        let lines = vec![
            b"INFO: hello".to_vec(),
            b"ERROR: bad".to_vec(),
            b"INFO: world".to_vec(),
        ];
        let result = apply_action(&CompiledAction::KeepLines(re), lines);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], b"ERROR: bad");
    }

    #[test]
    fn test_strip_ansi() {
        let re = Regex::new(ANSI_PATTERN).unwrap();
        let lines = vec![b"\x1B[31mred text\x1B[0m".to_vec()];
        let result = apply_action(&CompiledAction::StripAnsi(re), lines);
        assert_eq!(result[0], b"red text");
    }

    #[test]
    fn test_remove_empty_lines() {
        let lines = vec![
            b"hello".to_vec(),
            b"".to_vec(),
            b"   ".to_vec(),
            b"world".to_vec(),
        ];
        let result = apply_action(&CompiledAction::RemoveEmptyLines, lines);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_head() {
        let lines = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
        let result = apply_action(&CompiledAction::Head(2), lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], b"a");
        assert_eq!(result[1], b"b");
    }

    #[test]
    fn test_tail() {
        let lines = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
        let result = apply_action(&CompiledAction::Tail(2), lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], b"c");
        assert_eq!(result[1], b"d");
    }
}
