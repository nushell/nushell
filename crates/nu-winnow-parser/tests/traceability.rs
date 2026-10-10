//! The traceability matrix in `src/docs/11-traceability.md` must be
//! complete and must only point at things that exist.
//!
//! * Every fixture pattern in the tables matches at least one file under
//!   `tests/fixtures/`.
//! * Every backticked name in the `tests` column is a test function in
//!   `tests/*.rs` or `src/`.
//! * With a Nushell checkout (`NU_WINNOW_NUSHELL`, or the enclosing
//!   workspace at `../..`), every `SyntaxShape`, `FlatShape`, `TokenContents` and
//!   `ParseError` variant, every keyword command and every `pub fn parse_*`
//!   of `nu-parser` appears in the matching table, so that a new construct
//!   upstream fails here until it is mapped. This check is ignored by a plain
//!   `cargo test`, so that a Nushell change elsewhere does not fail CI for this
//!   crate: `verify.nu` runs it (`cargo test --test traceability --
//!   --include-ignored`).
#![allow(clippy::unwrap_used, clippy::format_push_string, reason = "tests may panic on failure")]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const DOC: &str = include_str!("../src/docs/11-traceability.md");

struct Row {
    section: String,
    item: String,
    fixtures: Vec<String>,
    tests: Vec<String>,
}

fn backticked(cell: &str) -> Vec<String> {
    cell.split('`').skip(1).step_by(2).map(str::to_string).collect()
}

/// The rows of every `| nu-parser | here | fixtures | tests |` table.
fn rows() -> Vec<Row> {
    let mut section = String::new();
    let mut out = Vec::new();
    for line in DOC.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            section = h.trim().to_string();
            continue;
        }
        if !line.starts_with("| `") && !line.starts_with("| ")
            || line.starts_with("| ---")
            || line.starts_with("| nu-parser")
        {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        let [item, _here, fixtures, tests] = cells.as_slice() else {
            panic!("{section}: a row does not have four cells: {line}");
        };
        let item = match item.strip_prefix('`').and_then(|i| i.strip_suffix('`')) {
            Some(name) => name.to_string(),
            None => item.to_string(),
        };
        out.push(Row { section: section.clone(), item, fixtures: backticked(fixtures), tests: backticked(tests) });
    }
    assert!(out.len() > 200, "the matrix should have hundreds of rows, found {}", out.len());
    out
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// `true` if `pattern` (a path with `*` wildcards in its last segment, or
/// `*.nu` in a directory) matches at least one fixture file.
fn pattern_matches(pattern: &str) -> bool {
    let root = fixture_root();
    let (dir, file) = pattern.rsplit_once('/').unwrap_or(("", pattern));
    let Ok(entries) = std::fs::read_dir(root.join(dir)) else { return false };
    entries.flatten().any(|e| glob_matches(file, &e.file_name().to_string_lossy()))
}

/// `*` matches any run of characters, any number of times in the pattern.
fn glob_matches(pattern: &str, name: &str) -> bool {
    let pieces: Vec<&str> = pattern.split('*').collect();
    let [first, middle @ .., last] = pieces.as_slice() else { return pattern == name };
    let Some(mut rest) = name.strip_prefix(first) else { return false };
    for piece in middle {
        let Some(at) = rest.find(piece) else { return false };
        rest = &rest[at + piece.len()..];
    }
    rest.ends_with(last) && rest.len() >= last.len()
}

/// Every `fn name` in the test files and the crate source.
fn test_functions() -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut names = BTreeSet::new();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root.join("tests")).unwrap().flatten() {
        if entry.path().extension().is_some_and(|e| e == "rs") {
            files.push(entry.path());
        }
    }
    collect_rs(&root.join("src"), &mut files);
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for line in text.lines() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("fn ").or_else(|| line.strip_prefix("pub fn ")) {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                names.insert(name);
            }
        }
    }
    names
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_fixture_pattern_and_test_name_exists() {
    let functions = test_functions();
    let mut problems = Vec::new();
    for row in rows() {
        assert!(!row.fixtures.is_empty(), "{}: `{}` names no fixture", row.section, row.item);
        assert!(!row.tests.is_empty(), "{}: `{}` names no test", row.section, row.item);
        for pattern in &row.fixtures {
            if !pattern_matches(pattern) {
                problems.push(format!("{}: `{}`: no fixture matches {pattern}", row.section, row.item));
            }
        }
        for test in &row.tests {
            if !functions.contains(test) {
                problems.push(format!("{}: `{}`: no test function named {test}", row.section, row.item));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

fn nushell_checkout() -> Option<PathBuf> {
    let candidate = match std::env::var_os("NU_WINNOW_NUSHELL") {
        Some(p) => PathBuf::from(p),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    };
    candidate.join("crates/nu-parser/src/lib.rs").exists().then_some(candidate)
}

/// The variant names of `pub enum NAME { .. }` in `text`.
fn enum_variants(text: &str, name: &str) -> Vec<String> {
    let start = text.find(&format!("pub enum {name}")).unwrap_or_else(|| panic!("no enum {name}"));
    let mut out = Vec::new();
    // Depth 1 is the enum body; brackets inside string literals do not nest.
    let mut depth = 0i32;
    for line in text[start..].lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let code = without_string_literals(line);
        let trimmed = code.trim();
        if depth == 1 && trimmed.chars().next().is_some_and(|c| c.is_ascii_uppercase()) && !trimmed.starts_with("Self")
        {
            out.push(trimmed.chars().take_while(|c| c.is_alphanumeric()).collect());
        }
        depth += code.matches('{').count() as i32 + code.matches('(').count() as i32;
        depth -= code.matches('}').count() as i32 + code.matches(')').count() as i32;
        if depth == 0 && code.contains('}') {
            break;
        }
    }
    out
}

fn without_string_literals(line: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for c in line.chars() {
        match (in_string, c) {
            (true, '\\') if !escaped => escaped = true,
            (true, '"') if !escaped => in_string = false,
            (true, _) => escaped = false,
            (false, '"') => in_string = true,
            (false, c) => out.push(c),
        }
    }
    out
}

/// `(section, upstream names)` pairs extracted from the checkout.
fn upstream(nushell: &Path) -> BTreeMap<&'static str, Vec<String>> {
    let read = |rel: &str| std::fs::read_to_string(nushell.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    let mut out = BTreeMap::new();
    out.insert("SyntaxShape", enum_variants(&read("crates/nu-protocol/src/syntax_shape.rs"), "SyntaxShape"));
    out.insert("FlatShape", enum_variants(&read("crates/nu-parser/src/flatten.rs"), "FlatShape"));
    out.insert("TokenContents", enum_variants(&read("crates/nu-parser/src/lex.rs"), "TokenContents"));
    out.insert("ParseError", enum_variants(&read("crates/nu-protocol/src/errors/parse_error.rs"), "ParseError"));

    let mut parse_fns = BTreeSet::new();
    let mut files = Vec::new();
    collect_rs(&nushell.join("crates/nu-parser/src"), &mut files);
    for file in &files {
        for line in std::fs::read_to_string(file).unwrap().lines() {
            if let Some(rest) = line.strip_prefix("pub fn parse_") {
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                parse_fns.insert(format!("parse_{name}"));
            }
        }
    }
    out.insert("Parser entry points", parse_fns.into_iter().collect());

    let mut keywords = BTreeSet::new();
    let mut files = Vec::new();
    collect_rs(&nushell.join("crates/nu-cmd-lang/src/core_commands"), &mut files);
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        if !text.contains("CommandType::Keyword") {
            continue;
        }
        if let Some(at) = text.find("fn name(&self)")
            && let Some(open) = text[at..].find('"')
            && let Some(close) = text[at + open + 1..].find('"')
        {
            keywords.insert(text[at + open + 1..at + open + 1 + close].to_string());
        }
    }
    out.insert("Keyword commands", keywords.into_iter().collect());
    out
}

#[test]
#[ignore = "upstream completeness: run by tools/scripts/verify.nu (`cargo test --test traceability -- --include-ignored`)"]
fn every_upstream_construct_is_mapped() {
    let Some(nushell) = nushell_checkout() else {
        eprintln!("no Nushell checkout; set NU_WINNOW_NUSHELL to run the upstream completeness check");
        return;
    };
    let rows = rows();
    let mut missing = Vec::new();
    for (section, names) in upstream(&nushell) {
        assert!(names.len() > 5, "{section}: extracted only {names:?}");
        let mapped: BTreeSet<&str> = rows.iter().filter(|r| r.section == section).map(|r| r.item.as_str()).collect();
        assert!(!mapped.is_empty(), "no table for {section}");
        for name in names {
            if !mapped.contains(name.as_str()) {
                missing.push(format!("{section}: `{name}` is not in src/docs/11-traceability.md"));
            }
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}
