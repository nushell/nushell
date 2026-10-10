//! Parse real-world Nushell files.
//!
//! `tests/corpus/` holds files from the Nushell standard library, the default
//! configuration files and `nu_scripts`. Set `NU_WINNOW_CORPUS=/path/to/dir`
//! to additionally parse every `.nu` file below that directory (for example a
//! checkout of <https://github.com/nushell/nu_scripts>); files listed in
//! `NU_WINNOW_CORPUS_SKIP` (comma-separated substrings) are ignored.
#![allow(clippy::unwrap_used, clippy::format_push_string, reason = "tests may panic on failure")]

use std::path::{Path, PathBuf};

use nu_winnow_parser::ast::{Expr, Visitor, walk_expression};
use nu_winnow_parser::{ParseConfig, parse_lenient};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "nu") {
            out.push(path);
        }
    }
}

struct Counter {
    exprs: usize,
    garbage: usize,
}

impl<'a> Visitor<'a> for Counter {
    fn visit_expression(&mut self, expr: &nu_winnow_parser::ast::Expression<'a>) {
        self.exprs += 1;
        if matches!(expr.expr, Expr::Garbage) {
            self.garbage += 1;
        }
        walk_expression(self, expr);
    }
}

fn check_dir(dir: &Path, skip: &[String]) -> (usize, Vec<String>) {
    let mut files = Vec::new();
    collect(dir, &mut files);
    files.sort();
    let config = ParseConfig::new();
    let mut failures = Vec::new();
    for file in &files {
        let name = file.display().to_string();
        if skip.iter().any(|s| !s.is_empty() && name.contains(s.as_str())) {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(file) else { continue };
        let (ast, diagnostics) = parse_lenient(&src, &config);
        let mut counter = Counter { exprs: 0, garbage: 0 };
        counter.visit_block(&ast.block);
        if let Some(d) = diagnostics.first() {
            failures.push(format!("{name}: {}", d.render(&src, Some(&name))));
        } else {
            assert_eq!(counter.garbage, 0, "{name}: garbage nodes without diagnostics");
        }
    }
    (files.len(), failures)
}

#[test]
fn embedded_corpus_parses_cleanly() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus");
    let (count, failures) = check_dir(&dir, &[]);
    assert!(count >= 10, "expected corpus files in {}", dir.display());
    assert!(failures.is_empty(), "{} of {count} files failed:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn external_corpus_parses_cleanly() {
    let Ok(dir) = std::env::var("NU_WINNOW_CORPUS") else {
        eprintln!("NU_WINNOW_CORPUS not set; skipping");
        return;
    };
    let skip: Vec<String> =
        std::env::var("NU_WINNOW_CORPUS_SKIP").unwrap_or_default().split(',').map(String::from).collect();
    let (count, failures) = check_dir(Path::new(&dir), &skip);
    eprintln!("{count} files in {dir}, {} failed", failures.len());
    assert!(failures.is_empty(), "{} of {count} files failed:\n{}", failures.len(), failures.join("\n"));
}
