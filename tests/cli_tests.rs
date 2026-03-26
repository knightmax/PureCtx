use assert_cmd::Command;
use predicates::prelude::*;

// ── Helper ─────────────────────────────────────────────────────────────────

/// Create a `pure` command from the test binary.
fn cmd() -> Command {
    Command::cargo_bin("pure").expect("binary `pure` not found")
}

// ── Proxy mode ─────────────────────────────────────────────────────────────

#[test]
fn proxy_runs_echo_command() {
    cmd()
        .args(["echo", "hello world"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world"));
}

#[test]
fn proxy_no_command_shows_error() {
    cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("no command specified"));
}

#[test]
fn proxy_nonexistent_command_fails() {
    cmd()
        .args(["nonexistent_command_xyz_12345"])
        .assert()
        .failure();
}

#[test]
fn proxy_preserves_exit_code_on_error() {
    // `false` exits with code 1
    cmd().args(["false"]).assert().failure();
}

// ── Filter management ──────────────────────────────────────────────────────

#[test]
fn filter_list_shows_builtins() {
    cmd()
        .args(["filter", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("maven"))
        .stdout(predicate::str::contains("npm"))
        .stdout(predicate::str::contains("cargo"))
        .stdout(predicate::str::contains("dotnet"))
        .stdout(predicate::str::contains("gradle"));
}

#[test]
fn filter_show_displays_filter() {
    cmd()
        .args(["filter", "show", "maven"])
        .assert()
        .success()
        .stdout(predicate::str::contains("maven"))
        .stdout(predicate::str::contains("pipeline"));
}

#[test]
fn filter_show_unknown_fails() {
    cmd()
        .args(["filter", "show", "nonexistent_filter_xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn filter_add_nonexistent_file_fails() {
    cmd()
        .args(["filter", "add", "/tmp/nonexistent_filter_xyz.toml"])
        .assert()
        .failure();
}

// ── Gain dashboard ─────────────────────────────────────────────────────────

#[test]
fn gain_shows_empty_dashboard() {
    cmd()
        .args(["gain"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Token Savings Dashboard"))
        .stdout(predicate::str::contains("Bronze"));
}

#[test]
fn gain_json_outputs_valid_json() {
    cmd()
        .args(["gain", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("summary"))
        .stdout(predicate::str::contains("daily"))
        .stdout(predicate::str::contains("by_command"));
}

#[test]
fn gain_csv_outputs_header() {
    cmd()
        .args(["gain", "--csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("period,commands,input_tokens"));
}

#[test]
fn gain_daily_works() {
    cmd().args(["gain", "--daily"]).assert().success();
}

#[test]
fn gain_weekly_works() {
    cmd().args(["gain", "--weekly"]).assert().success();
}

#[test]
fn gain_monthly_works() {
    cmd().args(["gain", "--monthly"]).assert().success();
}

#[test]
fn gain_top_works() {
    cmd().args(["gain", "--top", "5"]).assert().success();
}

#[test]
fn gain_history_works() {
    cmd().args(["gain", "--history", "10"]).assert().success();
}

// ── unit tests (via library) ───────────────────────────────────────────────

#[cfg(test)]
mod unit {
    use purectx::application::PurificationEngine;
    use purectx::domain::Purifier;
    use purectx::domain::clean::{CleanOptions, CleanPurifier};
    use purectx::domain::filter::FilterFile;
    use purectx::domain::sift::SiftPurifier;
    use purectx::domain::snip::SnipPurifier;
    use purectx::domain::stats::StatsPurifier;
    use purectx::infra::builtin::load_builtin_filters;
    use std::io::Cursor;

    fn run_engine(purifiers: Vec<Box<dyn Purifier>>, input: &[u8]) -> Vec<u8> {
        let reader = Cursor::new(input);
        let mut output = Vec::new();
        let engine = PurificationEngine::new(reader, &mut output, purifiers);
        engine.run().expect("engine failed");
        output
    }

    // ── FilterFile tests ───────────────────────────────────────────────────

    #[test]
    fn filter_parses_valid_toml() {
        let toml = r#"
name = "test"
version = 1
description = "Test filter"

[match]
command = "test"

[[pipeline]]
action = "remove_lines"
pattern = "^DEBUG"
"#;
        let filter = FilterFile::from_toml(toml).unwrap();
        assert_eq!(filter.name, "test");
        assert_eq!(filter.match_rules.command, "test");
        assert_eq!(filter.pipeline.len(), 1);
    }

    #[test]
    fn filter_rejects_empty_pipeline() {
        let toml = r#"
name = "empty"
version = 1

[match]
command = "test"

pipeline = []
"#;
        assert!(FilterFile::from_toml(toml).is_err());
    }

    #[test]
    fn filter_rejects_invalid_regex() {
        let toml = r#"
name = "bad"
version = 1

[match]
command = "test"

[[pipeline]]
action = "remove_lines"
pattern = "[invalid"
"#;
        assert!(FilterFile::from_toml(toml).is_err());
    }

    #[test]
    fn filter_matches_command() {
        let toml = r#"
name = "test"
version = 1

[match]
command = "mvn"
aliases = ["mvnw", "./mvnw"]

[[pipeline]]
action = "remove_empty_lines"
"#;
        let filter = FilterFile::from_toml(toml).unwrap();
        assert!(filter.matches("mvn", &[]));
        assert!(filter.matches("mvnw", &[]));
        assert!(filter.matches("./mvnw", &[]));
        assert!(!filter.matches("npm", &[]));
    }

    #[test]
    fn filter_matches_with_path() {
        let toml = r#"
name = "test"
version = 1

[match]
command = "mvn"

[[pipeline]]
action = "remove_empty_lines"
"#;
        let filter = FilterFile::from_toml(toml).unwrap();
        assert!(filter.matches("/usr/bin/mvn", &[]));
    }

    #[test]
    fn builtin_filters_load_successfully() {
        let filters = load_builtin_filters().unwrap();
        assert!(filters.len() >= 5);
        let names: Vec<&str> = filters.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"maven"));
        assert!(names.contains(&"npm"));
        assert!(names.contains(&"cargo"));
        assert!(names.contains(&"dotnet"));
        assert!(names.contains(&"gradle"));
    }

    // ── SiftPurifier unit tests ────────────────────────────────────────────

    #[test]
    fn sift_include_filters_correctly() {
        let purifier = SiftPurifier::new(Some("hello"), None).unwrap();
        assert_eq!(
            purifier.purify(b"hello world"),
            Some(b"hello world".to_vec())
        );
        assert_eq!(purifier.purify(b"goodbye"), None);
    }

    #[test]
    fn sift_exclude_filters_correctly() {
        let purifier = SiftPurifier::new(None, Some("skip")).unwrap();
        assert_eq!(purifier.purify(b"keep this"), Some(b"keep this".to_vec()));
        assert_eq!(purifier.purify(b"skip this"), None);
    }

    #[test]
    fn sift_both_include_and_exclude() {
        let purifier = SiftPurifier::new(Some("fn"), Some("test")).unwrap();
        // matches include, does not match exclude → keep
        assert!(purifier.purify(b"fn main()").is_some());
        // matches include AND exclude → drop
        assert!(purifier.purify(b"fn test_foo()").is_none());
        // does not match include → drop
        assert!(purifier.purify(b"let x = 1;").is_none());
    }

    #[test]
    fn sift_invalid_include_regex_returns_error() {
        assert!(SiftPurifier::new(Some("[invalid"), None).is_err());
    }

    // ── SnipPurifier unit tests ────────────────────────────────────────────

    #[test]
    fn snip_exclusive_extracts_only_inner_lines() {
        let purifier = SnipPurifier::new("BEGIN", "END", false).unwrap();
        assert_eq!(purifier.purify(b"before"), None);
        assert_eq!(purifier.purify(b"BEGIN"), None); // delimiter not included
        assert_eq!(purifier.purify(b"inside"), Some(b"inside".to_vec()));
        assert_eq!(purifier.purify(b"END"), None); // delimiter not included
        assert_eq!(purifier.purify(b"after"), None);
    }

    #[test]
    fn snip_inclusive_includes_delimiters() {
        let purifier = SnipPurifier::new("BEGIN", "END", true).unwrap();
        assert_eq!(purifier.purify(b"before"), None);
        assert_eq!(purifier.purify(b"BEGIN"), Some(b"BEGIN".to_vec()));
        assert_eq!(purifier.purify(b"inside"), Some(b"inside".to_vec()));
        assert_eq!(purifier.purify(b"END"), Some(b"END".to_vec()));
        assert_eq!(purifier.purify(b"after"), None);
    }

    // ── CleanPurifier unit tests ───────────────────────────────────────────

    #[test]
    fn clean_strips_double_slash_comment() {
        let purifier = CleanPurifier::new(CleanOptions {
            remove_comments: true,
            remove_empty_lines: false,
            minify_indent: false,
        });
        assert_eq!(
            purifier.purify(b"let x = 1; // comment"),
            Some(b"let x = 1;".to_vec())
        );
    }

    #[test]
    fn clean_strips_block_comment_single_line() {
        let purifier = CleanPurifier::new(CleanOptions {
            remove_comments: true,
            remove_empty_lines: false,
            minify_indent: false,
        });
        let result = purifier.purify(b"int x = /* foo */ 1;").unwrap();
        assert_eq!(result, b"int x =  1;");
    }

    #[test]
    fn clean_strips_multiline_block_comment() {
        let purifier = CleanPurifier::new(CleanOptions {
            remove_comments: true,
            remove_empty_lines: false,
            minify_indent: false,
        });
        // Line that opens a block comment – trailing content before `/*` is kept.
        assert_eq!(purifier.purify(b"code /* start"), Some(b"code".to_vec()));
        // Line inside the block comment → dropped
        assert_eq!(purifier.purify(b"still inside"), None);
        // Line that closes the block comment – content after `*/` is kept.
        assert_eq!(
            purifier.purify(b"end */ more code"),
            Some(b" more code".to_vec())
        );
    }

    #[test]
    fn clean_removes_empty_line() {
        let purifier = CleanPurifier::new(CleanOptions {
            remove_comments: false,
            remove_empty_lines: true,
            minify_indent: false,
        });
        assert_eq!(purifier.purify(b""), None);
        assert_eq!(purifier.purify(b"   "), None);
    }

    #[test]
    fn clean_minifies_indentation() {
        let purifier = CleanPurifier::new(CleanOptions {
            remove_comments: false,
            remove_empty_lines: false,
            minify_indent: true,
        });
        assert_eq!(
            purifier.purify(b"    indented"),
            Some(b" indented".to_vec())
        );
    }

    // ── StatsPurifier unit tests ───────────────────────────────────────────

    #[test]
    fn stats_passes_lines_unchanged() {
        let purifier = StatsPurifier::new();
        assert_eq!(purifier.purify(b"hello"), Some(b"hello".to_vec()));
        assert_eq!(purifier.purify(b"world"), Some(b"world".to_vec()));
    }

    // ── PurificationEngine unit tests ──────────────────────────────────────

    #[test]
    fn engine_applies_purifiers_in_order() {
        let purifiers: Vec<Box<dyn Purifier>> = vec![
            Box::new(SiftPurifier::new(None, Some("skip")).unwrap()),
            Box::new(CleanPurifier::new(CleanOptions::default())),
        ];
        let input = b"keep this\nskip this\n    indented keep\n";
        let output = run_engine(purifiers, input);
        let s = std::str::from_utf8(&output).unwrap();
        assert!(s.contains("keep this"));
        assert!(!s.contains("skip this"));
        assert!(s.contains("indented keep"));
    }

    #[test]
    fn engine_handles_empty_input() {
        let purifiers: Vec<Box<dyn Purifier>> =
            vec![Box::new(SiftPurifier::new(None, None).unwrap())];
        let output = run_engine(purifiers, b"");
        assert!(output.is_empty());
    }

    #[test]
    fn engine_handles_no_trailing_newline() {
        let purifiers: Vec<Box<dyn Purifier>> =
            vec![Box::new(SiftPurifier::new(None, None).unwrap())];
        let output = run_engine(purifiers, b"no newline");
        assert_eq!(output, b"no newline\n");
    }
}
