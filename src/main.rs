use anyhow::{Context, Result};
use clap::Parser;

use purectx::domain::Purifier;
use purectx::domain::clean::CleanOptions;
use purectx::domain::clean::CleanPurifier;
use purectx::domain::sift::SiftPurifier;
use purectx::domain::snip::SnipPurifier;
use purectx::domain::stats::StatsPurifier;
use purectx::infra::cli::{Cli, Commands};
use purectx::infra::io::run_stdio;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

/// Parse arguments, build the purifier list, and run the engine.
fn run() -> Result<()> {
    let cli = Cli::parse();

    let purifiers: Vec<Box<dyn Purifier>> = match cli.command {
        Commands::Sift(args) => {
            let sift = SiftPurifier::new(args.include.as_deref(), args.exclude.as_deref())
                .context("failed to build sift purifier")?;
            vec![Box::new(sift)]
        }

        Commands::Snip(args) => {
            let snip = SnipPurifier::new(&args.start, &args.end, args.inclusive)
                .context("failed to build snip purifier")?;
            vec![Box::new(snip)]
        }

        Commands::Clean(args) => {
            let opts = CleanOptions {
                remove_comments: !args.no_comments,
                remove_empty_lines: !args.no_empty_lines,
                minify_indent: !args.no_minify_indent,
            };
            vec![Box::new(CleanPurifier::new(opts))]
        }

        Commands::Stats => {
            vec![Box::new(StatsPurifier::new())]
        }
    };

    run_stdio(purifiers)
}
