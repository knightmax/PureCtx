# PureCtx Workspace Instructions

**PureCtx** is a command-output purification proxy for Large Language Models, written in Rust. These instructions help AI agents understand the project structure, conventions, and productivity patterns.

## Quick Navigation

| Purpose | File |
|---------|------|
| **Project guidelines** | [PureCtx.instructions.md](./PureCtx.instructions.md) (architecture, domain patterns, testing, code style) |
| **Rust conventions** | [instructions/rust.instructions.md](./instructions/rust.instructions.md) (idiomatic Rust, error handling, lifetimes) |
| **Project README** | [../README.md](../README.md) |

Don't duplicate content — these instructions link rather than embed.

## Build & Test Commands

Essential commands for local development:

```bash
# Build all targets (binary + library)
cargo build --all-targets

# Run integration tests
cargo test --all-targets

# Format code
cargo fmt

# Lint code
cargo clippy

# Build release binary
cargo build --release
```

**CI/CD**: GitHub Actions runs these commands on every push (see `.github/workflows/ci.yml`).

## Project Structure

```
src/
├── lib.rs                    # Public API (three-layer architecture)
├── main.rs                   # CLI entry point
├── domain/                   # Business logic & abstractions
│   ├── mod.rs               # Purifier trait definition
│   ├── clean.rs             # ANSI code stripping
│   ├── filter.rs            # Filter TOML parsing & validation
│   ├── snip.rs              # Regex-based block extraction
│   ├── sift.rs              # Pattern matching
│   ├── stats.rs             # Output statistics
│   └── tracking.rs          # Token & noise tracking
├── application/             # Stream orchestration
│   └── mod.rs               # PurificationEngine (generic I/O)
└── infra/                   # CLI & system integration
    ├── cli.rs               # Argument parsing (clap)
    ├── io.rs                # Stdin/stdout wiring
    ├── config.rs            # Config directory & caching
    ├── builtin.rs           # Built-in filters registry
    ├── proxy.rs             # Command spawning & output capture
    ├── gain.rs              # Token savings dashboard
    └── filters/             # Built-in filter TOML files
        ├── maven.toml
        ├── npm.toml
        ├── cargo.toml
        ├── dotnet.toml
        └── gradle.toml
tests/
└── cli_tests.rs             # Integration tests (assert_cmd + predicates)
```

## Architecture: Three Layers

PureCtx follows **domain-driven design** with strict separation:

### 1. **Domain** (`src/domain/`)
- Core abstractions: `Purifier` trait
- Business logic: purifier implementations
- Error types: `SnipError`, `FilterError`, etc. (using `thiserror`)
- **No I/O, no CLI concerns**

### 2. **Application** (`src/application/`)
- `PurificationEngine`: stream processor (generic over `BufRead + Write`)
- Orchestrates purifiers sequentially
- Handles newline management, finalization
- **No CLI, no specific file I/O**

### 3. **Infrastructure** (`src/infra/`)
- CLI argument parsing (`cli.rs`)
- Stdin/stdout wiring (`io.rs`)
- Filter loading & caching (`config.rs`)
- Built-in filter registry (`builtin.rs`)
- Command spawning (`proxy.rs`)
- **Bridges user intent to domain logic**

## Key Patterns

### The `Purifier` Trait

Every filter implementation satisfies this contract:

```rust
pub trait Purifier: Send + Sync {
    /// Process a single line (without trailing newline).
    /// Return Some(bytes) to keep, None to discard.
    fn purify(&self, input: &[u8]) -> Option<Vec<u8>>;
    
    /// Called once after all lines are processed (optional finalization).
    fn finalize(&self) {}
}
```

### Interior Mutability for State

Purifiers that track state across lines (e.g., `SnipPurifier`) use `Cell<T>`:

```rust
pub struct SnipPurifier {
    state: Cell<SnipState>,  // Tracks inside/outside block
}
unsafe impl Sync for SnipPurifier {}  // Safe: single-threaded use
```

### Error Handling Strategy

- **Domain**: Use `thiserror::Error` enums with explicit variants (`SnipError::InvalidPattern`, etc.)
- **Application**: Use `anyhow::Result<T>` with `.context("message")` for context
- **Infrastructure**: Translate errors into user-facing messages

### Filter Configuration (TOML)

Built-in filters define matching rules and a pipeline of actions:

```toml
name = "maven"
[match]
command = "mvn"           # Exact match (not regex)
aliases = ["mvnw"]

[[pipeline]]
action = "strip_ansi"

[[pipeline]]
action = "remove_lines"
pattern = "^\\[INFO\\] Download"

[[pipeline]]
action = "remove_empty_lines"
```

**Key**: `match.command` is exact (not regex). For multi-binary support (mvn + mvnd), create separate filters.

## Testing

- **Test framework**: `assert_cmd` + `predicates` for CLI testing
- **Test location**: `tests/cli_tests.rs` (integration tests)
- **Test command**: `cargo test --all-targets`
- **What to test**: Proxy behavior, filter activation, output correctness, error cases, edge cases (SIGPIPE, empty input, etc.)

## Related Documentation

- **What we're building**: See [README.md](../README.md) for user-facing overview
- **Contributing**: No formal CONTRIBUTING.md yet; follow PureCtx.instructions.md
- **Architecture deep-dive**: See [PureCtx.instructions.md](./PureCtx.instructions.md)

## Anti-Patterns to Avoid

- ❌ Using `panic!()` outside tests
- ❌ Ignoring errors silently without documentation
- ❌ Mixing I/O details into domain logic
- ❌ Hardcoding regex patterns (use TOML filters instead)
- ❌ Incomplete doc comments on public items
- ❌ Filters without integration tests

## Common Pitfalls

1. **Filter matching is exact, not regex**: `match.command = "mvn"` matches exactly "mvn", not anything containing "mvn". For `mvnd` support, create a separate filter.
2. **SIGPIPE handling**: When piping output to `head -5`, the pipe closes early. This is expected and handled gracefully in `infra/io.rs`.
3. **Line format**: Purifiers receive bytes **without** trailing newlines (`\n`). The engine adds them back.
4. **State management**: Purifiers must be `Send + Sync`. Use `Cell<T>` for single-threaded state (documented with `unsafe impl Sync`).

## Next Actions

For more detailed guidance on:
- **Architecture & code patterns**: Read [PureCtx.instructions.md](./PureCtx.instructions.md)
- **Rust conventions**: Read [instructions/rust.instructions.md](./instructions/rust.instructions.md)
- **Adding a new purifier**: See section "When Adding a New Purifier" in PureCtx.instructions.md
- **Adding a new filter**: See section "When Adding a New Filter" in PureCtx.instructions.md
