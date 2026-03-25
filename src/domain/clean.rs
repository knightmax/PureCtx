use std::cell::Cell;

use memchr::memmem;

use crate::domain::Purifier;

/// Options controlling what the [`CleanPurifier`] strips from input.
#[derive(Debug, Clone)]
pub struct CleanOptions {
    /// Remove single-line comments (`//`, `#`, `--`).
    pub remove_comments: bool,
    /// Remove blank lines (lines that are empty or contain only whitespace).
    pub remove_empty_lines: bool,
    /// Collapse leading whitespace to a single space (or nothing if the line
    /// has no non-whitespace content).
    pub minify_indent: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            remove_comments: true,
            remove_empty_lines: true,
            minify_indent: true,
        }
    }
}

/// Cleans source code or text by stripping comments, blank lines, and
/// excessive indentation.
///
/// ### Comment styles supported
/// * `//`  – C, C++, Rust, Go, Java, JavaScript, TypeScript
/// * `#`   – Python, Ruby, Shell, TOML, YAML
/// * `--`  – SQL, Lua, Haskell
/// * `/* … */` – C-style block comments (possibly spanning multiple lines)
///
/// Interior mutability (`Cell`) tracks multi-line block-comment state because
/// the [`Purifier`] trait requires `&self`.
pub struct CleanPurifier {
    opts: CleanOptions,
    /// `true` when we are inside a `/* … */` block comment.
    in_block_comment: Cell<bool>,
}

impl CleanPurifier {
    /// Create a new `CleanPurifier` with the given options.
    pub fn new(opts: CleanOptions) -> Self {
        Self {
            opts,
            in_block_comment: Cell::new(false),
        }
    }
}

// `Cell<bool>` is `!Sync`; safe here because we operate single-threaded.
unsafe impl Sync for CleanPurifier {}

impl Purifier for CleanPurifier {
    /// Strip comments, empty lines, and/or excess indentation from a single
    /// line.
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>> {
        // Work with an owned buffer so we can mutate it.
        let mut buf: Vec<u8> = input.to_vec();

        // ── Handle block comments ──────────────────────────────────────────
        if self.opts.remove_comments {
            if self.in_block_comment.get() {
                // Look for the closing `*/`
                if let Some(pos) = memmem::find(&buf, b"*/") {
                    // Keep content after `*/`
                    buf = buf[pos + 2..].to_vec();
                    self.in_block_comment.set(false);
                } else {
                    // Entire line is inside a block comment → discard
                    return None;
                }
            }

            // Handle block comment opening on this line.
            while let Some(pos) = memmem::find(&buf, b"/*") {
                if let Some(close) = memmem::find(&buf[pos + 2..], b"*/") {
                    // Block comment opens and closes on the same line.
                    let after = buf[pos + 2 + close + 2..].to_vec();
                    buf.truncate(pos);
                    buf.extend_from_slice(&after);
                } else {
                    // Block comment opens but does NOT close on this line.
                    self.in_block_comment.set(true);
                    buf.truncate(pos);
                    break;
                }
            }
        }

        let buf: &[u8] = &buf;

        // ── Strip single-line comments ─────────────────────────────────────
        let buf = if self.opts.remove_comments {
            strip_single_line_comment(buf)
        } else {
            buf
        };

        // ── Trim trailing whitespace ───────────────────────────────────────
        let buf = trim_end(buf);

        // ── Minify leading indentation ─────────────────────────────────────
        if self.opts.minify_indent {
            let trimmed = trim_start(buf);
            if trimmed.is_empty() {
                // Was all whitespace or empty.
                if self.opts.remove_empty_lines {
                    return None;
                }
                return Some(Vec::new());
            }
            if trimmed.len() < buf.len() {
                // There was leading whitespace – compress to a single space.
                let mut result = Vec::with_capacity(trimmed.len() + 1);
                result.push(b' ');
                result.extend_from_slice(trimmed);
                return Some(result);
            }
            // No leading whitespace; fall through.
        }

        // ── Remove empty lines ─────────────────────────────────────────────
        if self.opts.remove_empty_lines && buf.is_empty() {
            return None;
        }

        Some(buf.to_vec())
    }
}

/// Strip a single-line comment from the end of a byte slice.
///
/// Recognized prefixes (in order): `//`, `--`, `#`.
/// The `//` finder skips occurrences that are preceded by `:` (e.g. URLs
/// like `https://`).
fn strip_single_line_comment(line: &[u8]) -> &[u8] {
    // `//` – skip URL patterns (preceded by `:`)
    if let Some(pos) = memmem::find(line, b"//")
        && (pos == 0 || line[pos - 1] != b':')
    {
        return trim_end(&line[..pos]);
    }
    // `--`
    if let Some(pos) = memmem::find(line, b"--") {
        return trim_end(&line[..pos]);
    }
    // `#`
    if let Some(pos) = memchr::memchr(b'#', line) {
        return trim_end(&line[..pos]);
    }
    line
}

/// Remove trailing ASCII whitespace.
fn trim_end(s: &[u8]) -> &[u8] {
    let mut end = s.len();
    while end > 0 && s[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &s[..end]
}

/// Remove leading ASCII whitespace.
fn trim_start(s: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < s.len() && s[start].is_ascii_whitespace() {
        start += 1;
    }
    &s[start..]
}
