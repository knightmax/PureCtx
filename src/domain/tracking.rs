use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Maximum age (in seconds) before records are cleaned up: 90 days.
const MAX_AGE_SECS: u64 = 90 * 24 * 3600;

/// A single tracking record for one filtered command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingRecord {
    /// The original command that was executed (e.g. `"mvn clean install"`).
    pub command: String,
    /// Name of the filter that was applied.
    pub filter_name: String,
    /// Number of input bytes (before filtering).
    pub input_bytes: u64,
    /// Number of output bytes (after filtering).
    pub output_bytes: u64,
    /// Estimated input tokens (input_bytes / 4).
    pub input_tokens: u64,
    /// Estimated output tokens (output_bytes / 4).
    pub output_tokens: u64,
    /// Tokens saved (input_tokens - output_tokens).
    pub saved_tokens: u64,
    /// Savings percentage (0.0–100.0).
    pub savings_pct: f64,
    /// Execution time in milliseconds.
    pub duration_ms: u64,
    /// Unix timestamp (seconds since epoch).
    pub timestamp: u64,
}

/// Persistent storage for tracking records, backed by a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrackingDb {
    pub records: Vec<TrackingRecord>,
}

impl TrackingRecord {
    /// Create a new tracking record from raw byte counts and duration.
    pub fn new(
        command: &str,
        filter_name: &str,
        input_bytes: u64,
        output_bytes: u64,
        duration_ms: u64,
    ) -> Self {
        let input_tokens = input_bytes / 4;
        let output_tokens = output_bytes / 4;
        let saved_tokens = input_tokens.saturating_sub(output_tokens);
        let savings_pct = if input_tokens > 0 {
            (saved_tokens as f64 / input_tokens as f64) * 100.0
        } else {
            0.0
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            command: command.to_string(),
            filter_name: filter_name.to_string(),
            input_bytes,
            output_bytes,
            input_tokens,
            output_tokens,
            saved_tokens,
            savings_pct,
            duration_ms,
            timestamp,
        }
    }
}

impl TrackingDb {
    /// Return the path to the tracking database file.
    ///
    /// `~/.local/share/purectx/tracking.json` on Linux.
    pub fn db_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir().context("unable to determine data directory")?;
        Ok(data_dir.join("purectx").join("tracking.json"))
    }

    /// Load the tracking database from disk.
    ///
    /// If the file does not exist, returns an empty database.
    pub fn load() -> Result<Self> {
        let path = Self::db_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("cannot read tracking db: {}", path.display()))?;
        let db: Self = serde_json::from_str(&content)
            .with_context(|| format!("invalid tracking db: {}", path.display()))?;
        Ok(db)
    }

    /// Load from a specific path (for testing).
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("cannot read tracking db: {}", path.display()))?;
        let db: Self = serde_json::from_str(&content)
            .with_context(|| format!("invalid tracking db: {}", path.display()))?;
        Ok(db)
    }

    /// Persist the tracking database to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::db_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create data directory: {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("failed to serialize tracking db")?;
        fs::write(&path, json)
            .with_context(|| format!("cannot write tracking db: {}", path.display()))?;
        Ok(())
    }

    /// Save to a specific path (for testing).
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create data directory: {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("failed to serialize tracking db")?;
        fs::write(path, json)
            .with_context(|| format!("cannot write tracking db: {}", path.display()))?;
        Ok(())
    }

    /// Append a record and persist.
    pub fn record(&mut self, rec: TrackingRecord) -> Result<()> {
        self.records.push(rec);
        self.cleanup();
        self.save()
    }

    /// Remove records older than 90 days.
    pub fn cleanup(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.records
            .retain(|r| now.saturating_sub(r.timestamp) < MAX_AGE_SECS);
    }

    /// Total number of filtered commands.
    pub fn total_commands(&self) -> usize {
        self.records.len()
    }

    /// Total tokens saved across all records.
    pub fn total_saved_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.saved_tokens).sum()
    }

    /// Total input tokens across all records.
    pub fn total_input_tokens(&self) -> u64 {
        self.records.iter().map(|r| r.input_tokens).sum()
    }

    /// Weighted average savings percentage.
    pub fn avg_savings_pct(&self) -> f64 {
        let total_input = self.total_input_tokens();
        if total_input == 0 {
            return 0.0;
        }
        (self.total_saved_tokens() as f64 / total_input as f64) * 100.0
    }

    /// Total execution time in seconds.
    pub fn total_time_secs(&self) -> f64 {
        let ms: f64 = self.records.iter().map(|r| r.duration_ms as f64).sum();
        if ms == 0.0 { 0.0 } else { ms / 1000.0 }
    }

    /// Efficiency tier label based on average savings.
    ///
    /// Uses metallic tier names for gamification:
    /// Platinum (≥90%), Diamond (≥70%), Gold (≥50%), Silver (≥30%), Bronze (<30%).
    pub fn efficiency_tier(&self) -> &'static str {
        let pct = self.avg_savings_pct();
        if pct >= 90.0 {
            "Platinum"
        } else if pct >= 70.0 {
            "Diamond"
        } else if pct >= 50.0 {
            "Gold"
        } else if pct >= 30.0 {
            "Silver"
        } else {
            "Bronze"
        }
    }

    /// Emoji associated with the current efficiency tier.
    pub fn tier_emoji(&self) -> &'static str {
        match self.efficiency_tier() {
            "Platinum" => "\u{1F3C6}", // 🏆
            "Diamond" => "\u{1F48E}",  // 💎
            "Gold" => "\u{1F947}",     // 🥇
            "Silver" => "\u{1F948}",   // 🥈
            _ => "\u{1F949}",          // 🥉
        }
    }

    /// Information about the next tier: `(next_tier_name, next_tier_emoji,
    /// threshold_pct)`. Returns `None` if already at the highest tier (Platinum).
    pub fn next_tier_info(&self) -> Option<(&'static str, &'static str, f64)> {
        match self.efficiency_tier() {
            "Platinum" => None,
            "Diamond" => Some(("Platinum", "\u{1F3C6}", 90.0)),
            "Gold" => Some(("Diamond", "\u{1F48E}", 70.0)),
            "Silver" => Some(("Gold", "\u{1F947}", 50.0)),
            _ => Some(("Silver", "\u{1F948}", 30.0)),
        }
    }

    /// Return top N commands by total tokens saved.
    pub fn top_commands(&self, n: usize) -> Vec<CommandStats> {
        let mut map: std::collections::HashMap<String, CommandStats> =
            std::collections::HashMap::new();
        for r in &self.records {
            let entry = map
                .entry(r.command.clone())
                .or_insert_with(|| CommandStats {
                    command: r.command.clone(),
                    runs: 0,
                    saved_tokens: 0,
                    input_tokens: 0,
                });
            entry.runs += 1;
            entry.saved_tokens += r.saved_tokens;
            entry.input_tokens += r.input_tokens;
        }
        let mut sorted: Vec<CommandStats> = map.into_values().collect();
        sorted.sort_by(|a, b| b.saved_tokens.cmp(&a.saved_tokens));
        sorted.truncate(n);
        sorted
    }

    /// Return the last N records (most recent first).
    pub fn history(&self, n: usize) -> Vec<&TrackingRecord> {
        let mut recs: Vec<&TrackingRecord> = self.records.iter().collect();
        recs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        recs.truncate(n);
        recs
    }

    /// Aggregate records by day (last `days` days).
    pub fn daily(&self, days: u64) -> Vec<PeriodStats> {
        self.aggregate_by_period(days * 86400, 86400, "%Y-%m-%d")
    }

    /// Aggregate records by week (last `weeks` weeks).
    pub fn weekly(&self, weeks: u64) -> Vec<PeriodStats> {
        self.aggregate_by_period(weeks * 7 * 86400, 7 * 86400, "week")
    }

    /// Aggregate records by month (last `months` months, ~30-day buckets).
    pub fn monthly(&self, months: u64) -> Vec<PeriodStats> {
        self.aggregate_by_period(months * 30 * 86400, 30 * 86400, "month")
    }

    /// Generic period aggregation.
    fn aggregate_by_period(
        &self,
        window_secs: u64,
        bucket_secs: u64,
        label_prefix: &str,
    ) -> Vec<PeriodStats> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cutoff = now.saturating_sub(window_secs);

        let num_buckets = (window_secs / bucket_secs) as usize;
        let mut buckets: Vec<PeriodStats> = (0..num_buckets)
            .map(|i| {
                let bucket_start = now.saturating_sub((i as u64 + 1) * bucket_secs);
                PeriodStats {
                    period: if label_prefix == "%Y-%m-%d" {
                        format_date(bucket_start)
                    } else {
                        format!("{} -{}", label_prefix, i + 1)
                    },
                    commands: 0,
                    input_tokens: 0,
                    output_tokens: 0,
                    saved_tokens: 0,
                    savings_pct: 0.0,
                }
            })
            .collect();

        for r in &self.records {
            if r.timestamp < cutoff {
                continue;
            }
            let age = now.saturating_sub(r.timestamp);
            let bucket_idx = (age / bucket_secs) as usize;
            if bucket_idx < buckets.len() {
                let b = &mut buckets[bucket_idx];
                b.commands += 1;
                b.input_tokens += r.input_tokens;
                b.output_tokens += r.output_tokens;
                b.saved_tokens += r.saved_tokens;
            }
        }

        // Compute savings_pct for each bucket.
        for b in &mut buckets {
            b.savings_pct = if b.input_tokens > 0 {
                (b.saved_tokens as f64 / b.input_tokens as f64) * 100.0
            } else {
                0.0
            };
        }

        // Reverse so oldest is first.
        buckets.reverse();
        // Remove empty leading buckets.
        while buckets.first().is_some_and(|b| b.commands == 0) {
            buckets.remove(0);
        }
        buckets
    }
}

/// Aggregated stats for a single command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStats {
    pub command: String,
    pub runs: u64,
    pub saved_tokens: u64,
    pub input_tokens: u64,
}

impl CommandStats {
    /// Savings percentage for this command.
    pub fn savings_pct(&self) -> f64 {
        if self.input_tokens == 0 {
            return 0.0;
        }
        (self.saved_tokens as f64 / self.input_tokens as f64) * 100.0
    }
}

/// Aggregated stats for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodStats {
    pub period: String,
    pub commands: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub saved_tokens: u64,
    pub savings_pct: f64,
}

/// Format a unix timestamp as a YYYY-MM-DD date string.
fn format_date(ts: u64) -> String {
    // Simple date formatting without external crate.
    // Uses a basic algorithm to convert unix timestamp to date.
    let secs = ts as i64;
    let days = secs / 86400;
    // Days since 1970-01-01
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Convert days since 1970-01-01 to (year, month, day).
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Format a token count in human-readable form (e.g. "1.2K", "3.4M").
pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracking_record_new() {
        let rec = TrackingRecord::new("mvn clean install", "maven", 10000, 2000, 500);
        assert_eq!(rec.input_bytes, 10000);
        assert_eq!(rec.output_bytes, 2000);
        assert_eq!(rec.input_tokens, 2500);
        assert_eq!(rec.output_tokens, 500);
        assert_eq!(rec.saved_tokens, 2000);
        assert!((rec.savings_pct - 80.0).abs() < 0.1);
        assert_eq!(rec.duration_ms, 500);
        assert!(rec.timestamp > 0);
    }

    #[test]
    fn test_tracking_record_zero_input() {
        let rec = TrackingRecord::new("echo", "none", 0, 0, 100);
        assert_eq!(rec.savings_pct, 0.0);
    }

    #[test]
    fn test_tracking_db_persistence() {
        let tmp = std::env::temp_dir().join("purectx_test_db.json");
        // Clean up from previous runs.
        let _ = fs::remove_file(&tmp);

        let mut db = TrackingDb::default();
        let rec = TrackingRecord::new("cargo test", "cargo", 8000, 1000, 300);
        db.records.push(rec);
        db.save_to(&tmp).unwrap();

        let loaded = TrackingDb::load_from(&tmp).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].command, "cargo test");

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_summary_stats() {
        let mut db = TrackingDb::default();
        db.records
            .push(TrackingRecord::new("cmd1", "f1", 4000, 400, 100));
        db.records
            .push(TrackingRecord::new("cmd2", "f2", 8000, 800, 200));

        assert_eq!(db.total_commands(), 2);
        assert_eq!(db.total_input_tokens(), 3000); // 1000 + 2000
        assert_eq!(db.total_saved_tokens(), 2700); // 900 + 1800
        assert!((db.avg_savings_pct() - 90.0).abs() < 0.1);
        assert_eq!(db.efficiency_tier(), "Platinum");
    }

    #[test]
    fn test_top_commands() {
        let mut db = TrackingDb::default();
        // cmd1 run twice, cmd2 run once
        db.records
            .push(TrackingRecord::new("cmd1", "f", 4000, 400, 100));
        db.records
            .push(TrackingRecord::new("cmd1", "f", 4000, 400, 100));
        db.records
            .push(TrackingRecord::new("cmd2", "f", 8000, 800, 200));

        let top = db.top_commands(10);
        assert_eq!(top.len(), 2);
        // cmd2 saved more total tokens (1800 vs 1800 for cmd1, but cmd1 has 2 runs)
        // cmd1: 2 * 900 = 1800, cmd2: 1 * 1800 = 1800. Equal, order may vary.
        assert!(top.iter().any(|c| c.command == "cmd1" && c.runs == 2));
        assert!(top.iter().any(|c| c.command == "cmd2" && c.runs == 1));
    }

    #[test]
    fn test_history() {
        let mut db = TrackingDb::default();
        let mut rec1 = TrackingRecord::new("cmd1", "f", 4000, 400, 100);
        rec1.timestamp = 1000;
        let mut rec2 = TrackingRecord::new("cmd2", "f", 8000, 800, 200);
        rec2.timestamp = 2000;
        db.records.push(rec1);
        db.records.push(rec2);

        let hist = db.history(1);
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].command, "cmd2"); // most recent
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5K");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_efficiency_tiers() {
        let mut db = TrackingDb::default();
        db.records
            .push(TrackingRecord::new("cmd", "f", 4000, 400, 100)); // 90%
        assert_eq!(db.efficiency_tier(), "Platinum");
        assert_eq!(db.tier_emoji(), "\u{1F3C6}"); // 🏆
        assert!(db.next_tier_info().is_none()); // max tier

        let mut db2 = TrackingDb::default();
        db2.records
            .push(TrackingRecord::new("cmd", "f", 4000, 1200, 100)); // 70%
        assert_eq!(db2.efficiency_tier(), "Diamond");
        assert_eq!(db2.tier_emoji(), "\u{1F48E}"); // 💎
        assert_eq!(db2.next_tier_info().unwrap().0, "Platinum");

        let mut db3 = TrackingDb::default();
        db3.records
            .push(TrackingRecord::new("cmd", "f", 4000, 2000, 100)); // 50%
        assert_eq!(db3.efficiency_tier(), "Gold");
        assert_eq!(db3.tier_emoji(), "\u{1F947}"); // 🥇
        assert_eq!(db3.next_tier_info().unwrap().0, "Diamond");

        let mut db4 = TrackingDb::default();
        db4.records
            .push(TrackingRecord::new("cmd", "f", 4000, 2800, 100)); // 30%
        assert_eq!(db4.efficiency_tier(), "Silver");
        assert_eq!(db4.tier_emoji(), "\u{1F948}"); // 🥈
        assert_eq!(db4.next_tier_info().unwrap().0, "Gold");

        let mut db5 = TrackingDb::default();
        db5.records
            .push(TrackingRecord::new("cmd", "f", 4000, 3200, 100)); // 20%
        assert_eq!(db5.efficiency_tier(), "Bronze");
        assert_eq!(db5.tier_emoji(), "\u{1F949}"); // 🥉
        assert_eq!(db5.next_tier_info().unwrap().0, "Silver");
    }

    #[test]
    fn test_cleanup_removes_old_records() {
        let mut db = TrackingDb::default();
        let mut old_rec = TrackingRecord::new("old", "f", 4000, 400, 100);
        old_rec.timestamp = 1; // very old
        db.records.push(old_rec);
        db.records
            .push(TrackingRecord::new("recent", "f", 4000, 400, 100));

        db.cleanup();
        assert_eq!(db.records.len(), 1);
        assert_eq!(db.records[0].command, "recent");
    }

    #[test]
    fn test_format_date() {
        // 2026-01-01 00:00:00 UTC = 1767225600
        assert_eq!(format_date(1767225600), "2026-01-01");
        // 2026-03-26 = 1774483200
        assert_eq!(format_date(1774483200), "2026-03-26");
    }
}
