use assert_cmd::Command;
use predicates::prelude::*;

// ── Helper ─────────────────────────────────────────────────────────────────

/// Create a `pure` command from the test binary.
fn cmd() -> Command {
    Command::cargo_bin("pure").expect("binary `pure` not found")
}

// ── sift ───────────────────────────────────────────────────────────────────

#[test]
fn sift_include_keeps_matching_lines() {
    cmd()
        .args(["sift", "--include", "TODO"])
        .write_stdin("// TODO: fix this\nlet x = 1;\n// TODO: later\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("let x = 1;").not());
}

#[test]
fn sift_exclude_removes_matching_lines() {
    cmd()
        .args(["sift", "--exclude", "^#"])
        .write_stdin("# comment\ncode line\n# another\n")
        .assert()
        .success()
        .stdout("code line\n");
}

#[test]
fn sift_include_and_exclude_combined() {
    cmd()
        .args(["sift", "--include", "fn", "--exclude", "test"])
        .write_stdin("fn main() {}\nfn test_it() {}\npub fn run() {}\n")
        .assert()
        .success()
        .stdout("fn main() {}\npub fn run() {}\n");
}

#[test]
fn sift_no_args_passes_everything() {
    cmd()
        .args(["sift"])
        .write_stdin("hello\nworld\n")
        .assert()
        .success()
        .stdout("hello\nworld\n");
}

#[test]
fn sift_invalid_regex_exits_nonzero() {
    cmd()
        .args(["sift", "--include", "[invalid"])
        .write_stdin("")
        .assert()
        .failure();
}

// ── snip ───────────────────────────────────────────────────────────────────

#[test]
fn snip_exclusive_extracts_block() {
    let input = "before\nBEGIN\nline1\nline2\nEND\nafter\n";
    cmd()
        .args(["snip", "--start", "BEGIN", "--end", "END"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("line1\nline2\n");
}

#[test]
fn snip_inclusive_includes_delimiters() {
    let input = "before\nBEGIN\nline1\nEND\nafter\n";
    cmd()
        .args(["snip", "--start", "BEGIN", "--end", "END", "--inclusive"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("BEGIN\nline1\nEND\n");
}

#[test]
fn snip_multiple_blocks() {
    let input = "A\nBEGIN\nx\nEND\nB\nBEGIN\ny\nEND\nC\n";
    cmd()
        .args(["snip", "--start", "BEGIN", "--end", "END"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout("x\ny\n");
}

// ── clean ──────────────────────────────────────────────────────────────────

#[test]
fn clean_removes_line_comments() {
    cmd()
        .args(["clean", "--no-empty-lines", "--no-minify-indent"])
        .write_stdin("let x = 1; // assign x\n")
        .assert()
        .success()
        .stdout("let x = 1;\n");
}

#[test]
fn clean_removes_hash_comments() {
    cmd()
        .args(["clean", "--no-empty-lines", "--no-minify-indent"])
        .write_stdin("x = 1  # python\n")
        .assert()
        .success()
        .stdout("x = 1\n");
}

#[test]
fn clean_removes_block_comments_single_line() {
    cmd()
        .args(["clean", "--no-empty-lines", "--no-minify-indent"])
        .write_stdin("int x = /* secret */ 1;\n")
        .assert()
        .success()
        .stdout("int x =  1;\n");
}

#[test]
fn clean_removes_empty_lines() {
    cmd()
        .args(["clean", "--no-comments", "--no-minify-indent"])
        .write_stdin("a\n\nb\n   \nc\n")
        .assert()
        .success()
        .stdout("a\nb\nc\n");
}

#[test]
fn clean_minifies_indent() {
    cmd()
        .args(["clean", "--no-comments", "--no-empty-lines"])
        .write_stdin("    indented\n")
        .assert()
        .success()
        .stdout(" indented\n");
}

#[test]
fn clean_preserves_urls() {
    // `https://` must NOT be treated as a `//` comment
    cmd()
        .args(["clean", "--no-minify-indent"])
        .write_stdin("// see https://example.com for details\n")
        .assert()
        .success()
        // The whole line is a comment starting with //; the resulting empty
        // line is then removed by the default remove_empty_lines=true.
        .stdout("");
}

// ── stats ──────────────────────────────────────────────────────────────────

#[test]
fn stats_passes_data_through() {
    cmd()
        .args(["stats"])
        .write_stdin("hello\nworld\n")
        .assert()
        .success()
        .stdout("hello\nworld\n");
}

#[test]
fn stats_writes_to_stderr() {
    cmd()
        .args(["stats"])
        .write_stdin("hello\nworld\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("[stats]"));
}

// ── unit tests (via library) ───────────────────────────────────────────────

#[cfg(test)]
mod unit {
    use purectx::application::PurificationEngine;
    use purectx::domain::Purifier;
    use purectx::domain::clean::{CleanOptions, CleanPurifier};
    use purectx::domain::sift::SiftPurifier;
    use purectx::domain::snip::SnipPurifier;
    use purectx::domain::stats::StatsPurifier;
    use std::io::Cursor;

    fn run_engine(purifiers: Vec<Box<dyn Purifier>>, input: &[u8]) -> Vec<u8> {
        let reader = Cursor::new(input);
        let mut output = Vec::new();
        let engine = PurificationEngine::new(reader, &mut output, purifiers);
        engine.run().expect("engine failed");
        output
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
