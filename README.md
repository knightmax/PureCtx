# PureCtx

[![CI](https://github.com/knightmax/PureCtx/actions/workflows/ci.yml/badge.svg)](https://github.com/knightmax/PureCtx/actions/workflows/ci.yml)

**`pure`** is a command-output purification proxy for Large Language Models,
written in Rust. Place it in front of **any** command to automatically filter
noise from its output — reducing token counts and improving signal-to-noise
ratio before feeding logs into an LLM.

Tools produce output designed for **humans** (colors, progress bars, download
logs, verbose status lines). An LLM only needs the **essentials**. `pure`
removes the noise.

## Quick Start

```bash
# Instead of:
mvn clean install

# Run through pure:
pure mvn clean install

# Works with any command:
pure npm run build
pure cargo test
pure dotnet build
pure gradle assemble
```

## How It Works

1. `pure` **wraps** the given command (spawns it as a child process)
2. Captures **stdout** and **stderr**
3. Auto-detects the command and selects a matching **filter**
4. Applies the filter pipeline to strip noise
5. Outputs the purified result

If no filter matches, output is passed through unchanged.

## Built-in Filters

| Filter   | Commands                    | What it removes                              |
|----------|-----------------------------|----------------------------------------------|
| `maven`  | `mvn`, `mvnw`, `./mvnw`    | Download progress, separators, empty [INFO]  |
| `npm`    | `npm`, `npx`, `pnpm`, `yarn` | Warnings, notices, timing, funding messages |
| `cargo`  | `cargo`                     | Compiling, Downloading, Checking progress    |
| `dotnet` | `dotnet`                    | Restore progress, "Build succeeded" noise    |
| `gradle` | `gradle`, `gradlew`        | Download progress, task headers              |

All built-in filters strip ANSI escape codes (colors) and remove empty lines.

## Custom Filters

Custom filters are TOML files stored in `~/.config/purectx/filters/`.
They take **priority** over built-in filters.

### Installing a custom filter

```bash
pure filter add my-tool.toml
```

### Listing all filters

```bash
pure filter list
```

### Showing a filter's definition

```bash
pure filter show maven
```

### Filter file format

```toml
name = "my-tool"
version = 1
description = "Reduces my-tool output noise"

[match]
command = "my-tool"
aliases = ["mt"]

[[pipeline]]
action = "strip_ansi"

[[pipeline]]
action = "remove_lines"
pattern = "^\\[DEBUG\\]"

[[pipeline]]
action = "remove_lines"
pattern = "^\\s*Progress:"

[[pipeline]]
action = "remove_empty_lines"
```

### Pipeline actions

| Action              | Description                            | Parameters    |
|---------------------|----------------------------------------|---------------|
| `remove_lines`      | Remove lines matching a regex          | `pattern`     |
| `keep_lines`        | Keep only lines matching a regex       | `pattern`     |
| `strip_ansi`        | Remove ANSI escape sequences (colors)  | —             |
| `remove_empty_lines`| Remove blank lines                     | —             |
| `head`              | Keep only the first N lines            | `count`       |
| `tail`              | Keep only the last N lines             | `count`       |

### Match rules

| Field        | Required | Description                                 |
|--------------|----------|---------------------------------------------|
| `command`    | yes      | The base command name (e.g. `"mvn"`)        |
| `aliases`    | no       | Alternative names (e.g. `["mvnw"]`)         |
| `subcommand` | no      | First argument must match (e.g. `"test"`)   |

### Error handling

```toml
on_error = "passthrough"  # default: output raw result if filter fails
on_error = "fail"          # propagate the error
```

## Gain Dashboard

`pure` tracks token savings across all filtered commands with a gamified
dashboard. View savings with:

```bash
pure gain
```

```
  ┌─────────────────────────────────────────┐
  │  🧹 PureCtx — Token Savings Dashboard  │
  └─────────────────────────────────────────┘

  ╔═════════════════════════════╗
  ║     🏆     Platinum     🏆     ║
  ╚═════════════════════════════╝

  📊 Summary
  ───────────────────────────────
  🔢 Commands filtered     3
  💰 Tokens saved          32.8K
  📉 Avg savings           93.6%
  ⏱️  Total time            20.3s

  🎯 Progress
  ───────────────────────────────
  🏆 ███████████████████░ 94%  Platinum
  ✨ Max level reached!

  🏅 Top commands by tokens saved
  ───────────────────────────────
  Command                         Runs     Saved  Savings  Impact
  ──────────────────────────────  ─────  ────────  ───────  ────────────
  🥇 cargo test                      1     19.5K    97.5%  ████████████
  🥈 mvn clean install               1      9.0K    90.0%  ██████░░░░░░
  🥉 npm run build                   1      4.2K    85.0%  ███░░░░░░░░░
```

### Views

| Flag | Description |
|------|-------------|
| *(none)* | Full dashboard: tier badge + KPIs + progress + top 10 commands |
| `--daily` | Daily breakdown (last 7 days) |
| `--weekly` | Weekly breakdown (last 8 weeks) |
| `--monthly` | Monthly breakdown (last 6 months) |
| `--top N` | Top N commands by tokens saved |
| `--history N` | Last N filtered commands |
| `--json` | Full report as JSON (summary + daily + by_command) |
| `--csv` | Daily stats as CSV |

### Tier system

| Tier | Emoji | Threshold |
|------|-------|-----------|
| 🏆 Platinum | 🏆 | ≥ 90% |
| 💎 Diamond | 💎 | ≥ 70% |
| 🥇 Gold | 🥇 | ≥ 50% |
| 🥈 Silver | 🥈 | ≥ 30% |
| 🥉 Bronze | 🥉 | < 30% |

### Tracking database

Savings are stored in `~/.local/share/purectx/tracking.json`. Records older
than 90 days are automatically cleaned up.

**What's tracked:** command, filter name, input/output bytes and tokens, savings
percentage, execution time, timestamp.

**What's NOT tracked:** passthrough commands (no matching filter) and commands
with 0 input bytes.

## Installation

```bash
cargo install --path .
```

The binary is named **`pure`**.

## Architecture

The project follows Clean Architecture principles:

```
src/
├── domain/          # Core abstractions
│   ├── mod.rs       #   Purifier trait
│   ├── filter.rs    #   FilterFile model (TOML-backed)
│   ├── tracking.rs  #   TrackingDb + TrackingRecord (JSON-backed)
│   ├── sift.rs      #   SiftPurifier – regex filtering
│   ├── snip.rs      #   SnipPurifier – block extraction
│   ├── clean.rs     #   CleanPurifier – comment & whitespace removal
│   └── stats.rs     #   StatsPurifier – token statistics
├── application/
│   └── mod.rs       # PurificationEngine – drives the stream pipeline
├── infra/
│   ├── cli.rs       # Clap v4 CLI definitions
│   ├── io.rs        # stdin/stdout adapter with SIGPIPE handling
│   ├── proxy.rs     # Command proxy (spawn, capture, filter, track)
│   ├── gain.rs      # Gain dashboard display + export (JSON/CSV)
│   ├── config.rs    # Filter directory management (~/.config/purectx/)
│   ├── builtin.rs   # Built-in filter loader
│   └── filters/     # Embedded TOML filter definitions
│       ├── maven.toml
│       ├── npm.toml
│       ├── cargo.toml
│       ├── dotnet.toml
│       └── gradle.toml
├── lib.rs           # Library re-exports (enables testability)
└── main.rs          # Binary entry point
tests/
└── cli_tests.rs     # Integration + unit tests (assert_cmd)
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap 4` | CLI argument parsing (derive macros) |
| `regex 1` | Byte-level regular expressions |
| `memchr 2` | Fast byte / substring search |
| `serde 1` | Filter TOML + tracking JSON serialization |
| `serde_json 1` | JSON serialization for tracking db + export |
| `toml 0.8` | TOML parser |
| `dirs 5` | Platform config/data directory (`~/.config/`, `~/.local/share/`) |
| `thiserror 1` | Ergonomic error types |
| `anyhow 1` | Error propagation in the application layer |

Dev: `assert_cmd 2`, `predicates 3`.

## Testing

```bash
cargo test
```

The test suite covers:
- Unit tests for all purifiers and the engine
- Filter TOML parsing and matching
- Pipeline actions (remove/keep lines, ANSI strip, head/tail)
- Tracking database persistence, KPIs, cleanup, and formatting
- CLI integration tests (proxy, filter management, gain dashboard)

## License

MIT
