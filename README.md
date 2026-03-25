# PureCtx

[![CI](https://github.com/knightmax/PureCtx/actions/workflows/ci.yml/badge.svg)](https://github.com/knightmax/PureCtx/actions/workflows/ci.yml)

**`pure`** is a command-line context-purification utility for Large Language Models,
written in Rust. It reads from stdin and writes a cleaned, filtered version of
the input to stdout – reducing token counts and improving signal-to-noise ratio
before feeding text into an LLM.

## Features

| Subcommand | Description |
|------------|-------------|
| `sift`  | Filter lines by regular expression (include / exclude) |
| `snip`  | Extract structural blocks between start and end patterns |
| `clean` | Remove comments, blank lines and excess indentation |
| `stats` | Report byte and estimated token counts to stderr |

## Installation

```bash
cargo install --path .
```

The binary is named **`pure`**.

## Usage

All subcommands read from **stdin** and write to **stdout**, making them fully
composable via Unix pipes:

```bash
# Keep only lines containing "TODO"
cat file.rs | pure sift --include "TODO"

# Remove lines starting with a `#`
cat script.sh | pure sift --exclude "^#"

# Extract everything between `BEGIN` and `END` markers (exclusive)
cat file.txt | pure snip --start "BEGIN" --end "END"

# Extract inclusive of the delimiter lines
cat file.txt | pure snip --start "BEGIN" --end "END" --inclusive

# Strip comments, blank lines and excess indentation (all on by default)
cat file.rs | pure clean

# Strip comments only, keep blank lines and original indentation
cat file.rs | pure clean --no-empty-lines --no-minify-indent

# Count bytes / estimated tokens in the final output
cat file.rs | pure clean | pure stats

# Full pipeline
cat large_file.rs \
  | pure sift --exclude "^//" \
  | pure clean \
  | pure stats
```

### SIGPIPE

`pure` handles broken pipes gracefully (e.g. `pure | head -5`) – the process
exits cleanly without printing an error.

## Architecture

The project follows Clean Architecture principles:

```
src/
├── domain/          # Core abstractions (Purifier trait + implementations)
│   ├── mod.rs       #   Purifier trait
│   ├── sift.rs      #   SiftPurifier – regex filtering
│   ├── snip.rs      #   SnipPurifier – block extraction
│   ├── clean.rs     #   CleanPurifier – comment & whitespace removal
│   └── stats.rs     #   StatsPurifier – token statistics
├── application/
│   └── mod.rs       # PurificationEngine – drives the stream pipeline
├── infra/
│   ├── cli.rs       # Clap v4 CLI definitions
│   └── io.rs        # stdin/stdout adapter with SIGPIPE handling
├── lib.rs           # Library re-exports (enables testability)
└── main.rs          # Binary entry point
tests/
└── cli_tests.rs     # Integration + unit tests (assert_cmd)
```

### `Purifier` trait

```rust
pub trait Purifier: Send + Sync {
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>>;
    fn finalize(&self) {}
}
```

* `purify` is called once per line (without the trailing `\n`).
  Returning `Some(bytes)` keeps the line; returning `None` drops it.
* `finalize` is called once after the stream ends (used by `StatsPurifier`
  to print to stderr).

### `PurificationEngine`

```rust
pub struct PurificationEngine<R: BufRead, W: Write> { … }
```

Accepts any `BufRead` + `Write` pair – inject `std::io::Cursor<&[u8]>` /
`Vec<u8>` for unit testing without real I/O.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap 4` | CLI argument parsing (derive macros) |
| `regex 1` | Byte-level regular expressions |
| `memchr 2` | Fast byte / substring search |
| `thiserror 1` | Ergonomic error types |
| `anyhow 1` | Error propagation in the application layer |

Dev: `assert_cmd 2`, `predicates 3`.

## Testing

```bash
cargo test
```

The test suite covers unit tests for every purifier and end-to-end integration
tests for all subcommands.

## License

MIT
