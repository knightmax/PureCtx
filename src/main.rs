use anyhow::{Context, Result, bail};
use clap::Parser;

use purectx::domain::filter::FilterFile;
use purectx::infra::builtin::load_builtin_filters;
use purectx::infra::cli::{Cli, Commands, FilterAction};
use purectx::infra::config;
use purectx::infra::proxy::run_proxy;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

/// Parse arguments, resolve filters, and either proxy a command or manage
/// filters.
fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle the `filter` management subcommand.
    if let Some(Commands::Filter(cmd)) = cli.command {
        return handle_filter_command(cmd.action);
    }

    // Otherwise, treat everything as an external command to proxy.
    if cli.external.is_empty() {
        bail!(
            "no command specified. Usage: pure <COMMAND> [ARGS...]\n\nRun `pure --help` for more information."
        );
    }

    let command = &cli.external[0];
    let args: Vec<String> = cli.external[1..].to_vec();

    // Load all filters (built-in + custom) and find one matching the command.
    let filter = find_matching_filter(command, &args)?;

    if let Some(ref f) = filter {
        eprintln!("[pure] using filter: {} ({})", f.name, f.description);
    }

    let exit_code = run_proxy(command, &args, filter.as_ref())?;
    std::process::exit(exit_code);
}

/// Find the first filter that matches the given command and arguments.
///
/// Custom filters take priority over built-in ones.
fn find_matching_filter(command: &str, args: &[String]) -> Result<Option<FilterFile>> {
    // Custom filters first (they override built-in ones).
    let customs = config::load_custom_filters().context("failed to load custom filters")?;
    for f in customs {
        if f.matches(command, args) {
            return Ok(Some(f));
        }
    }

    // Then built-in filters.
    let builtins = load_builtin_filters().context("failed to load built-in filters")?;
    for f in builtins {
        if f.matches(command, args) {
            return Ok(Some(f));
        }
    }

    Ok(None)
}

/// Handle `pure filter {add|list|show}` management commands.
fn handle_filter_command(action: FilterAction) -> Result<()> {
    match action {
        FilterAction::Add(args) => {
            let name = config::add_filter(&args.file).context("failed to install filter")?;
            eprintln!("Filter `{name}` installed successfully.");
            Ok(())
        }
        FilterAction::List => {
            let filters = config::list_filters()?;
            if filters.is_empty() {
                println!("No filters available.");
            } else {
                println!("{:<20} {:<12} DESCRIPTION", "NAME", "SOURCE");
                println!("{}", "-".repeat(60));
                for (name, desc, source) in &filters {
                    println!("{:<20} {:<12} {}", name, source, desc);
                }
            }
            Ok(())
        }
        FilterAction::Show(args) => {
            // Try custom first, then built-in.
            let customs = config::load_custom_filters()?;
            for f in &customs {
                if f.name == args.name {
                    println!(
                        "{}",
                        toml::to_string_pretty(f).context("failed to serialize filter")?
                    );
                    return Ok(());
                }
            }
            let builtins = load_builtin_filters()?;
            for f in &builtins {
                if f.name == args.name {
                    println!(
                        "{}",
                        toml::to_string_pretty(f).context("failed to serialize filter")?
                    );
                    return Ok(());
                }
            }
            bail!("filter `{}` not found", args.name);
        }
    }
}
