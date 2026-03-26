use anyhow::Result;
use serde::Serialize;

use crate::domain::tracking::{TrackingDb, format_tokens};
use crate::infra::cli::GainArgs;

/// Handle the `pure gain` subcommand.
pub fn handle_gain(args: &GainArgs) -> Result<()> {
    let db = TrackingDb::load()?;

    if args.json {
        return print_json(&db);
    }

    if args.csv {
        return print_csv(&db);
    }

    if args.daily {
        return print_period_breakdown(&db, "daily", 7);
    }

    if args.weekly {
        return print_period_breakdown(&db, "weekly", 8);
    }

    if args.monthly {
        return print_period_breakdown(&db, "monthly", 6);
    }

    if let Some(n) = args.top {
        return print_top_commands(&db, n);
    }

    if let Some(n) = args.history {
        return print_history(&db, n);
    }

    // Default: full dashboard
    print_dashboard(&db)
}

/// Print the full gamified gain dashboard.
fn print_dashboard(db: &TrackingDb) -> Result<()> {
    println!();
    println!(
        "  \u{250C}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}"
    ); // ┌─────────────────────────────────────────┐
    println!("  \u{2502}  \u{1F9F9} PureCtx \u{2014} Token Savings Dashboard  \u{2502}"); // │  🧹 PureCtx — Token Savings Dashboard  │
    println!(
        "  \u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}"
    ); // └─────────────────────────────────────────┘
    println!();

    if db.records.is_empty() {
        println!("  \u{1F949} Level: Bronze"); // 🥉
        println!();
        println!("  No filtered commands recorded yet.");
        println!("  Run commands through pure to start earning tokens!");
        println!();
        println!("  Example: pure cargo test");
        println!();
        return Ok(());
    }

    // Tier badge
    let tier = db.efficiency_tier();
    let emoji = db.tier_emoji();
    println!(
        "  \u{2554}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2557}"
    ); // ╔═════════════════════════════╗
    println!("  \u{2551}     {emoji}  {tier:^17}  {emoji}     \u{2551}"); // ║     🏆      Platinum      🏆     ║
    println!(
        "  \u{255A}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{255D}"
    ); // ╚═════════════════════════════╝
    println!();

    // Summary KPIs with emojis
    println!("  \u{1F4CA} Summary"); // 📊
    println!(
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
    ); // ───────────────────────────────
    println!("  \u{1F522} Commands filtered     {}", db.total_commands()); // 🔢
    println!(
        "  \u{1F4B0} Tokens saved          {}",
        format_tokens(db.total_saved_tokens())
    ); // 💰
    println!(
        "  \u{1F4C9} Avg savings           {:.1}%",
        db.avg_savings_pct()
    ); // 📉
    println!(
        "  \u{23F1}\u{FE0F}  Total time            {:.1}s",
        db.total_time_secs()
    ); // ⏱️
    println!();

    // Progress bar with tier info
    let pct = db.avg_savings_pct().min(100.0);
    let filled = (pct / 5.0).round() as usize;
    let empty = 20_usize.saturating_sub(filled);
    println!("  \u{1F3AF} Progress"); // 🎯
    println!(
        "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
    ); // ───────────────────────────────
    println!(
        "  {emoji} {}{} {:.0}%  {tier}",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        pct,
    );

    // Next tier info
    if let Some((next_name, next_emoji, threshold)) = db.next_tier_info() {
        let remaining = threshold - pct;
        println!(
            "  \u{27A1}\u{FE0F}  Next: {next_emoji} {next_name} (need {remaining:.1}% more savings)"
        ); // ➡️
    } else {
        println!("  \u{2728} Max level reached!"); // ✨
    }
    println!();

    // Top commands
    let top = db.top_commands(10);
    if !top.is_empty() {
        println!("  \u{1F3C5} Top commands by tokens saved"); // 🏅
        println!(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}"
        ); // ───────────────────────────────
        println!(
            "  {:<30} {:>5}  {:>8}  {:>7}  Impact",
            "Command", "Runs", "Saved", "Savings"
        );
        println!(
            "  {}  {}  {}  {}  {}",
            "\u{2500}".repeat(30),
            "\u{2500}".repeat(5),
            "\u{2500}".repeat(8),
            "\u{2500}".repeat(7),
            "\u{2500}".repeat(12)
        );

        let max_saved = top.first().map(|c| c.saved_tokens).unwrap_or(1).max(1);
        for (i, cmd) in top.iter().enumerate() {
            let bar_len = ((cmd.saved_tokens as f64 / max_saved as f64) * 12.0).round() as usize;
            let bar_filled = "\u{2588}".repeat(bar_len);
            let bar_empty = "\u{2591}".repeat(12_usize.saturating_sub(bar_len));
            let medal = medal_emoji(i);
            let display_cmd = truncate(&cmd.command, CMD_DISPLAY_WIDTH);
            println!(
                "  {medal} {:<28} {:>5}  {:>8}  {:>6.1}%  {bar_filled}{bar_empty}",
                display_cmd,
                cmd.runs,
                format_tokens(cmd.saved_tokens),
                cmd.savings_pct(),
            );
        }
    }

    println!();
    Ok(())
}

/// Print a period breakdown table.
fn print_period_breakdown(db: &TrackingDb, kind: &str, count: u64) -> Result<()> {
    let periods = match kind {
        "daily" => db.daily(count),
        "weekly" => db.weekly(count),
        "monthly" => db.monthly(count),
        _ => Vec::new(),
    };

    if periods.is_empty() {
        println!("No data for {kind} breakdown.");
        return Ok(());
    }

    println!();
    println!(
        "  {:<15} {:>5}  {:>10}  {:>10}  {:>10}  {:>7}",
        "Period", "Cmds", "Input", "Output", "Saved", "Savings"
    );
    println!(
        "  {}  {}  {}  {}  {}  {}",
        "─".repeat(15),
        "─".repeat(5),
        "─".repeat(10),
        "─".repeat(10),
        "─".repeat(10),
        "─".repeat(7)
    );

    for p in &periods {
        println!(
            "  {:<15} {:>5}  {:>10}  {:>10}  {:>10}  {:>6.1}%",
            p.period,
            p.commands,
            format_tokens(p.input_tokens),
            format_tokens(p.output_tokens),
            format_tokens(p.saved_tokens),
            p.savings_pct,
        );
    }
    println!();
    Ok(())
}

/// Print top N commands table.
fn print_top_commands(db: &TrackingDb, n: usize) -> Result<()> {
    let top = db.top_commands(n);
    if top.is_empty() {
        println!("No commands recorded yet.");
        return Ok(());
    }

    let max_saved = top.first().map(|c| c.saved_tokens).unwrap_or(1).max(1);

    println!();
    println!(
        "  {:<30} {:>5}  {:>8}  {:>7}  Impact",
        "Command", "Runs", "Saved", "Savings"
    );
    println!(
        "  {}  {}  {}  {}  {}",
        "\u{2500}".repeat(30),
        "\u{2500}".repeat(5),
        "\u{2500}".repeat(8),
        "\u{2500}".repeat(7),
        "\u{2500}".repeat(12)
    );

    for (i, cmd) in top.iter().enumerate() {
        let bar_len = ((cmd.saved_tokens as f64 / max_saved as f64) * 12.0).round() as usize;
        let bar_filled = "\u{2588}".repeat(bar_len);
        let bar_empty = "\u{2591}".repeat(12_usize.saturating_sub(bar_len));
        let medal = medal_emoji(i);
        let display_cmd = truncate(&cmd.command, CMD_DISPLAY_WIDTH);
        println!(
            "  {medal} {:<28} {:>5}  {:>8}  {:>6.1}%  {bar_filled}{bar_empty}",
            display_cmd,
            cmd.runs,
            format_tokens(cmd.saved_tokens),
            cmd.savings_pct(),
        );
    }
    println!();
    Ok(())
}

/// Print command history.
fn print_history(db: &TrackingDb, n: usize) -> Result<()> {
    let history = db.history(n);
    if history.is_empty() {
        println!("No commands recorded yet.");
        return Ok(());
    }

    println!();
    println!(
        "  {:<30} {:>10}  {:>10}  {:>7}  {:>8}",
        "Command", "Input", "Output", "Savings", "Time"
    );
    println!(
        "  {}  {}  {}  {}  {}",
        "─".repeat(30),
        "─".repeat(10),
        "─".repeat(10),
        "─".repeat(7),
        "─".repeat(8)
    );

    for rec in &history {
        let display_cmd = truncate(&rec.command, 30);
        println!(
            "  {:<30} {:>10}  {:>10}  {:>6.1}%  {:>7.1}s",
            display_cmd,
            format_tokens(rec.input_tokens),
            format_tokens(rec.output_tokens),
            rec.savings_pct,
            rec.duration_ms as f64 / 1000.0,
        );
    }
    println!();
    Ok(())
}

/// JSON export structure.
#[derive(Serialize)]
struct JsonReport {
    summary: JsonSummary,
    daily: Vec<serde_json::Value>,
    by_command: Vec<serde_json::Value>,
}

#[derive(Serialize)]
struct JsonSummary {
    commands_filtered: usize,
    tokens_saved: u64,
    avg_savings_pct: f64,
    efficiency: String,
    total_time_secs: f64,
}

/// Print the full report as JSON.
fn print_json(db: &TrackingDb) -> Result<()> {
    let report = JsonReport {
        summary: JsonSummary {
            commands_filtered: db.total_commands(),
            tokens_saved: db.total_saved_tokens(),
            avg_savings_pct: db.avg_savings_pct(),
            efficiency: db.efficiency_tier().to_string(),
            total_time_secs: db.total_time_secs(),
        },
        daily: db
            .daily(7)
            .iter()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .collect(),
        by_command: db
            .top_commands(100)
            .iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect(),
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Print daily stats as CSV.
fn print_csv(db: &TrackingDb) -> Result<()> {
    println!("period,commands,input_tokens,output_tokens,saved_tokens,savings_pct");
    for p in db.daily(7) {
        println!(
            "{},{},{},{},{},{:.1}",
            p.period, p.commands, p.input_tokens, p.output_tokens, p.saved_tokens, p.savings_pct,
        );
    }
    Ok(())
}

/// Truncate a string to max length, adding "…" if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

/// Maximum display width for command names in medal-prefixed tables.
const CMD_DISPLAY_WIDTH: usize = 28;

/// Return a medal emoji for the given rank (0-indexed).
fn medal_emoji(rank: usize) -> &'static str {
    match rank {
        0 => "\u{1F947}", // 🥇
        1 => "\u{1F948}", // 🥈
        2 => "\u{1F949}", // 🥉
        _ => " ",
    }
}
