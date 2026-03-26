---
name: PureCtx Project Guidelines
description: "Use when: implementing features, refactoring, writing tests, or adding documentation to PureCtx. Follow this for consistent architecture, error handling, code style, and domain-driven design."
applyTo: "src/**/*.rs,tests/**/*.rs,README.md"
---

# PureCtx Development Guidelines

PureCtx is a **domain-driven**, **trait-based** stream processor for command-output purification. These guidelines enforce consistency across implementation, testing, and documentation.

## Architecture & Design

### Core Three-Layer Architecture

Maintain strict separation of concerns across three layers:

#### 1. **Domain Layer** (`src/domain/`)
- **What it contains**: Core abstractions, trait definitions, and business logic.
- **Pattern**: Each purifier type gets its own module (e.g., `snip.rs`, `filter.rs`, `clean.rs`).
- **Key trait**: `Purifier` — the central abstraction. All implementations must:
  - Accept `&[u8]` input (line without trailing newline)
  - Return `Option<Vec<u8>>` (keep/transform or discard)
  - Implement `Send + Sync` (thread-safe)
  - Use interior mutability (`Cell<T>`) for cross-line state
- **Error handling**: Use `thiserror::Error` for domain-specific errors with explicit variants.

#### 2. **Application Layer** (`src/application/`)
- **What it contains**: `PurificationEngine` — the stream orchestrator.
- **Pattern**: Agnostic to I/O details; works with any `BufRead` + `Write`.
- **Responsibility**: Applies purifiers sequentially (short-circuit on `None`), appends newlines, invokes `finalize()`.
- **Error handling**: Use `anyhow::Result` for application errors; add context via `.context("message")`.

#### 3. **Infrastructure Layer** (`src/infra/`)
- **What it contains**: CLI (`cli.rs`), I/O helpers (`io.rs`), config loading (`config.rs`, `builtin.rs`).
- **Pattern**: Bridges between user intent (CLI args) and domain logic (purifiers).
- **Responsibility**: Parse filters, instantiate purifiers, wire them into the engine.
- **Error handling**: Translate errors into user-facing messages.

### Trait-Based Abstractions

- **Always use traits** for concepts that will have multiple implementations (e.g., `Purifier`).
- **Keep traits small** — `Purifier` has two methods (`purify`, `finalize`); that's intentional.
- **Document trait expectations** in detail (e.g., input format, return semantics, thread safety).

### Interior Mutability for State

When a purifier needs to track state across lines (e.g., `SnipPurifier` tracking inside/outside blocks):
- Use `Cell<T>` (not `RefCell<T>` — no runtime borrow checks needed).
- Annotate `unsafe impl Sync` with a comment explaining single-threaded safety.
- Example:
  ```rust
  pub struct SnipPurifier {
      state: Cell<SnipState>,
  }
  unsafe impl Sync for SnipPurifier {}
  ```

## Code Style & Implementation

### Documentation

- **Every public item** must have a doc comment (`///`).
- **Functions**: Document arguments (`# Arguments`), return value (`# Returns`), and error cases (`# Errors`).
- **Traits**: Explain the contract clearly; document any assumptions (e.g., thread safety, line format).
- **Example** (from `domain/mod.rs`):
  ```rust
  /// A `Purifier` processes a single line of bytes and decides whether to keep
  /// it (possibly transformed) or discard it.
  ///
  /// Returning `Some(bytes)` keeps the line (with the provided content).
  /// Returning `None` discards the line entirely.
  pub trait Purifier: Send + Sync {
      fn purify(&self, input: &[u8]) -> Option<Vec<u8>>;
  }
  ```

### Error Handling

- **Domain errors**: Define explicit `enum` using `#[derive(Debug, Error)]` from `thiserror`.
  - Example: `SnipError`, `FilterError`.
  - Each variant has a `#[error("...")]` message.
  - Use `#[source]` to include the root cause.

- **Application errors**: Use `anyhow::Result<T>` and add context with `.context("...")`.
  - Example: `reader.read_until(...)?.context("failed to read from input")?`

- **Never silently ignore errors** unless documented (e.g., SIGPIPE handling in `infra/io.rs`).

### Filter Configuration

- **Format**: TOML files in `.github/infra/filters/` (e.g., `maven.toml`, `npm.toml`).
- **Structure**:
  ```toml
  name = "maven"
  version = 1
  description = "..."
  
  [match]
  command = "mvn"  # Exact match (not regex)
  aliases = ["mvnw", "./mvnw"]
  
  [[pipeline]]
  action = "remove_lines"
  pattern = "^\\[INFO\\] Download"
  ```
- **Semantics**:
  - `match.command` is **exact** (not regex). For multi-binary support (e.g., `mvn` + `mvnd`), create separate filters.
  - `pipeline` is an **ordered list** of actions applied sequentially.
  - Each action must have `pattern` defined; `inclusive` is optional.

## Testing

### Test Organization

- **Location**: `tests/cli_tests.rs` (integration tests).
- **Convention**: Use `assert_cmd` + `predicates` for CLI testing.
- **Structure**:
  ```rust
  #[test]
  fn test_name() {
      cmd()
          .args(["arg1", "arg2"])
          .assert()
          .success()  // or .failure()
          .stdout(predicate::str::contains("expected output"));
  }
  ```

### What to Test

- **Proxy mode**: Command execution, exit codes, output passthrough.
- **Filter management**: `filter list`, `filter show`, `filter add`.
- **Purification**: Built-in filters (maven, npm, cargo, dotnet, gradle) remove expected noise.
- **Error cases**: Missing command, invalid filter, malformed TOML.
- **Edge cases**: Empty input, very long lines, SIGPIPE scenarios.

### Test Helpers

- Use `cmd()` helper to get a fresh `Command` bound to the `pure` binary.
- Use `predicates` for assertions (e.g., `.contains()`, `.contains_regex()`).

## Documentation & Communication

### README

- **Purpose**: Explain what `pure` does, how to use it, and how it works (high-level).
- **Sections**:
  1. Brief description + badges (CI status).
  2. Quick Start examples.
  3. How It Works (architecture overview).
  4. Built-in Filters table.
  5. Custom Filters guide (link to filter format).
  6. Installation / Contributing.

### Module Documentation

- Each module (`domain`, `application`, `infra`) must have a summary at the top of `mod.rs`.
- Example:
  ```rust
  //! Infrastructure layer: CLI definitions and I/O helpers.
  ```

### Commit Messages

- Use imperative mood: "Add filter support" not "Added filter support".
- Reference issue numbers: "Fixes #42".
- Keep subject line ≤ 50 characters; body ≤ 72 characters per line.
- Example:
  ```
  Add clean purifier for removing ANSI codes
  
  Implements CleanPurifier trait to strip color and
  formatting from output. Fixes #15.
  ```

## Conventions

### Naming

- **Trait implementations**: `{Name}Purifier` (e.g., `SnipPurifier`, `CleanPurifier`).
- **Errors**: `{Module}Error` (e.g., `SnipError`, `FilterError`).
- **Modules**: lowercase, descriptive (e.g., `snip`, `filter`, `stats`).

### Module Responsibilities

| Module | Responsibility |
|--------|-----------------|
| `domain/mod.rs` | Core `Purifier` trait definition |
| `domain/{type}.rs` | One purifier implementation per file |
| `application/mod.rs` | `PurificationEngine` orchestration |
| `infra/cli.rs` | CLI argument parsing |
| `infra/io.rs` | Stdin/stdout wiring |
| `infra/config.rs` | Filter loading and caching |
| `infra/builtin.rs` | Built-in filter registry |

### Dependencies

- **Always minimize**: Only add dependencies if they solve a problem better than a simple implementation.
- **Current justified dependencies**:
  - `clap`: CLI parsing (complex arg handling).
  - `regex`: Pattern matching in filters.
  - `serde` + `toml`: Config serialization.
  - `thiserror`: Error macros.
  - `anyhow`: Error context.

## Workflow

### When Adding a New Purifier

1. Define the type in `domain/{name}.rs`.
2. Implement `Purifier` trait with full doc comments.
3. Use `thiserror::Error` for any errors.
4. Add tests in `tests/cli_tests.rs`.
5. Update `infra/builtin.rs` to register it (if built-in).
6. Update README if user-facing.

### When Adding a New Filter

1. Create a TOML file in `.github/infra/filters/{name}.toml`.
2. Follow the structure documented above.
3. Test with `pure filter add {file}` and verify it activates.
4. Add integration tests.
5. Document in README.

### Before Committing

- [ ] All tests pass: `cargo test`
- [ ] Build succeeds: `cargo build --release`
- [ ] Formatter is happy: `cargo fmt`
- [ ] Linter is happy: `cargo clippy`
- [ ] Doc comments are complete and accurate.
- [ ] Error messages are user-friendly.

## Anti-Patterns

- ❌ **Using `panic!`** outside of tests — use error types instead.
- ❌ **Ignoring errors silently** — always propagate or document why ignored.
- ❌ **Mixing I/O details into domain logic** — keep `Purifier` generic.
- ❌ **Regex patterns as hardcoded strings** — use TOML filters instead.
- ❌ **Incomplete doc comments** — every public item must be documented.
- ❌ **Writing filters without tests** — test every filter's behavior.
