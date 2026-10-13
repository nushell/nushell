//! Differential testing: run the same text through `nu-parser` (from the
//! local Nushell checkout) and `nu-winnow-parser`, in process, and report
//! where their verdicts differ.
//!
//! ```text
//! cargo run --release --bin differential -- [OPTIONS] [FILE|DIR ...]
//!   --snippets FILE.json   a `{origin, snippet}` list from tools/scripts/extract-corpus.nu (repeatable)
//!   --mutants N            also parse N random mutations of every input (default 0)
//!   --seed S               seed for the mutations (default 1)
//!   --no-std               do not load the standard library into nu-parser
//!   --details              print every disagreement with both messages
//!   --json                 print the summary as JSON instead of a table
//! ```
//!
//! nu-parser's errors are split into *syntactic* ones (delimiters, keywords,
//! literals, operators) and *semantic* ones that need declarations, types
//! or files (`VariableNotFound`, `UnknownFlag`, `ModuleNotFound`, ...). A
//! syntax-only parser can only be expected to agree with the former, so the
//! summary reports:
//!
//! * `agree`: both accept, or both reject;
//! * `ours_rejects`: nu-parser accepts and this parser rejects (a bug here);
//! * `nu_syntax_rejects`: nu-parser reports a syntax error and this parser
//!   accepts (a bug here, or a rule still to port);
//! * `nu_semantic_rejects`: nu-parser's first error is semantic and this
//!   parser accepts (the checks a consumer adds: declarations, types, files);
//! * `panics`: either parser panicked (always a bug).
//!
//! There is no list of tolerated differences: `--details` prints every row
//! that is not `agree`, semantic ones included, with both messages.
//!
//! The exit status is 1 when `ours_rejects`, `nu_syntax_rejects` or
//! `panics` is non-zero, so the run can gate a check-in.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use nu_protocol::ParseError;
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_winnow_parser::lex::{LexOptions, TokenContents, lex};
use nu_winnow_parser::{ParseConfig, parse_lenient};
use nushell_harness::engine;
use serde::Deserialize;

#[derive(Deserialize)]
struct Snippet {
    origin: String,
    snippet: String,
}

/// One text to compare, with where it came from.
struct Unit {
    origin: String,
    text: String,
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect(&entry.path(), out);
            }
        }
    } else if path.extension().is_some_and(|e| e == "nu") {
        out.push(path.to_path_buf());
    }
}

/// The variant name of a `ParseError`, from its `Debug` form.
fn variant(err: &ParseError) -> String {
    let text = format!("{err:?}");
    text.chars().take_while(|c| c.is_alphanumeric()).collect()
}

/// Errors nu-parser reports from the text alone, which a syntax-only parser
/// must reproduce. Everything else needs declarations, signatures, types or
/// files.
fn is_syntax_error(err: &ParseError) -> bool {
    let name = variant(err);
    // "expected int", "expected block, closure or record", "expected 1
    // closure parameter", "expected operator" after a complete positional:
    // the shape a signature asks for decided how the argument was read.
    if matches!(name.as_str(), "Expected" | "ExpectedWithStringMsg" | "Mismatch") {
        let message = err.to_string().to_lowercase();
        let shape_words = [
            "int",
            "float",
            "number",
            "string",
            "bool",
            "duration",
            "filesize",
            "datetime",
            "range",
            "cell path",
            "cell-path",
            "glob",
            "path",
            "directory",
            "binary",
            "list",
            "record",
            "table",
            "closure",
            "block",
            "one of",
            "non-block",
            "any",
            "nothing",
            "signature",
            "operator",
            "at least one range bound",
            ". or ! or ?",
        ];
        if let Some(rest) = message.split("expected ").nth(1)
            && (shape_words.iter().any(|w| rest.starts_with(w)) || rest.starts_with(|c: char| c.is_ascii_digit()))
        {
            return false;
        }
    }
    // A dangling operator inside a row condition (`any status ==`) is only
    // found because the positional's shape is a math expression.
    if name == "IncompleteMathExpression" {
        return false;
    }
    // An item read as an int because the signature asks for one (`first
    // 0x[01]`, which is binary anywhere else).
    if let ParseError::InvalidLiteral(_, entity, _) = err
        && entity == "int"
    {
        return false;
    }
    // `use a b c` where `b` is not a submodule of `a`: found when the import
    // pattern is resolved against the module's contents.
    if let ParseError::WrongImportPattern(message, _) = err
        && message.starts_with("Trying to import something but the parent")
    {
        return false;
    }
    matches!(
        name.as_str(),
        "ExtraTokens"
            | "ExtraTokensAfterClosingDelimiter"
            | "UnexpectedEof"
            | "Unclosed"
            | "Unbalanced"
            | "Expected"
            | "ExpectedWithStringMsg"
            | "ExpectedWithDidYouMean"
            | "Mismatch"
            | "ShellAndAnd"
            | "ShellOrOr"
            | "ShellErrRedirect"
            | "ShellOutErrRedirect"
            | "MultipleRedirections"
            | "ExpectedKeyword"
            | "UnexpectedKeyword"
            | "CantAliasKeyword"
            | "CantAliasExpression"
            | "UnknownOperator"
            | "AssignInPipeline"
            | "NameIsKeyword"
            | "InvalidBinaryString"
            | "MultipleRestParams"
            | "VariableNotValid"
            | "AliasNotValid"
            | "CommandDefNotValid"
            | "UnknownType"
            | "KeywordMissingArgument"
            | "MissingType"
            | "RestNeedsName"
            | "WrongImportPattern"
            | "InvalidLiteral"
            | "RedirectingBuiltinCommand"
            | "AssignmentRequiresVar"
            | "AttributeRequiresDefinition"
            | "BuiltinCommandInPipeline"
    )
}

enum Verdict {
    Accept,
    Reject(String),
    Panic,
}

fn nu_verdict(engine_state: &EngineState, name: &str, text: &str) -> (Verdict, bool) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut working_set = StateWorkingSet::new(engine_state);
        let _ = nu_parser::parse(&mut working_set, Some(name), text.as_bytes(), false);
        working_set.parse_errors.first().map(|e| (format!("{}: {e}", variant(e)), is_syntax_error(e)))
    }));
    match result {
        Err(_) => (Verdict::Panic, false),
        Ok(None) => (Verdict::Accept, false),
        Ok(Some((message, syntax))) => (Verdict::Reject(message), syntax),
    }
}

fn our_verdict(config: &ParseConfig, text: &str) -> Verdict {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let (_, diagnostics) = parse_lenient(text, config);
        diagnostics.first().map(|d| d.to_string())
    }));
    match result {
        Err(_) => Verdict::Panic,
        Ok(None) => Verdict::Accept,
        Ok(Some(message)) => Verdict::Reject(message),
    }
}

#[derive(Default)]
struct Summary {
    total: usize,
    agree: usize,
    ours_rejects: usize,
    nu_syntax_rejects: usize,
    nu_semantic_rejects: usize,
    panics: usize,
}

/// A small deterministic generator (SplitMix64) so runs are reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Token spans of `text` from this crate's lexer, or whitespace-separated
/// words when the text does not lex.
fn token_spans(text: &str) -> Vec<(usize, usize)> {
    match lex(text, 0, LexOptions::BLOCK) {
        Ok(tokens) => {
            tokens.iter().filter(|t| t.contents != TokenContents::Eof).map(|t| (t.span.start, t.span.end)).collect()
        }
        Err(_) => text
            .split_whitespace()
            .map(|w| {
                let start = w.as_ptr() as usize - text.as_ptr() as usize;
                (start, start + w.len())
            })
            .collect(),
    }
}

/// A random edit of `text`: delete, duplicate or swap a token, insert a
/// delimiter, drop a character or truncate. Each mutant is a program a user
/// could plausibly have typed halfway through an edit.
fn mutate(text: &str, rng: &mut Rng) -> String {
    let spans = token_spans(text);
    let boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect();
    let delimiters = ["(", ")", "[", "]", "{", "}", "\"", "'", "|", ";", "$", "..", ":", "=", "#", "\n"];
    match rng.below(if spans.is_empty() { 3 } else { 6 }) {
        0 => {
            let at = boundaries[rng.below(boundaries.len())];
            format!("{}{}{}", &text[..at], delimiters[rng.below(delimiters.len())], &text[at..])
        }
        1 => {
            let at = boundaries[rng.below(boundaries.len())];
            text[..at].to_string()
        }
        2 => {
            let i = rng.below(boundaries.len().saturating_sub(1).max(1));
            let (start, end) = (boundaries[i], boundaries.get(i + 1).copied().unwrap_or(text.len()));
            format!("{}{}", &text[..start], &text[end..])
        }
        3 => {
            let (start, end) = spans[rng.below(spans.len())];
            format!("{}{}", &text[..start], &text[end..])
        }
        4 => {
            let (start, end) = spans[rng.below(spans.len())];
            format!("{} {}{}", &text[..end], &text[start..end], &text[end..])
        }
        _ => {
            let i = rng.below(spans.len());
            let j = rng.below(spans.len());
            let (a, b) = (spans[i.min(j)], spans[i.max(j)]);
            if a == b || a.1 > b.0 {
                return text.to_string();
            }
            format!("{}{}{}{}{}", &text[..a.0], &text[b.0..b.1], &text[a.1..b.0], &text[a.0..a.1], &text[b.1..])
        }
    }
}

fn compare(engine_state: &EngineState, config: &ParseConfig, unit: &Unit, summary: &mut Summary, details: bool) {
    summary.total += 1;
    let (nu, syntax) = nu_verdict(engine_state, &unit.origin, &unit.text);
    let ours = our_verdict(config, &unit.text);
    let (class, report) = match (&nu, &ours) {
        (Verdict::Panic, _) | (_, Verdict::Panic) => (&mut summary.panics, Some("PANIC")),
        (Verdict::Accept, Verdict::Accept) | (Verdict::Reject(_), Verdict::Reject(_)) => (&mut summary.agree, None),
        (Verdict::Accept, Verdict::Reject(_)) => (&mut summary.ours_rejects, Some("ours rejects, nu accepts")),
        (Verdict::Reject(_), Verdict::Accept) if syntax => {
            (&mut summary.nu_syntax_rejects, Some("nu rejects (syntax), ours accepts"))
        }
        (Verdict::Reject(_), Verdict::Accept) => {
            (&mut summary.nu_semantic_rejects, Some("nu rejects (semantic), ours accepts"))
        }
    };
    *class += 1;
    if let Some(label) = report
        && details
    {
        let describe = |v: &Verdict| match v {
            Verdict::Accept => "accept".to_string(),
            Verdict::Reject(m) => m.clone(),
            Verdict::Panic => "PANIC".to_string(),
        };
        println!(
            "== {label}: {}\n   nu:   {}\n   ours: {}\n   text: {:?}",
            unit.origin,
            describe(&nu),
            describe(&ours),
            unit.text
        );
    }
}

pub fn main() {
    let mut paths = Vec::new();
    let mut snippets = Vec::new();
    let mut mutants = 0usize;
    let mut seed = 1u64;
    let mut std = true;
    let mut details = false;
    let mut json = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--snippets" => snippets.push(PathBuf::from(args.next().expect("--snippets FILE"))),
            "--mutants" => mutants = args.next().and_then(|n| n.parse().ok()).expect("--mutants N"),
            "--seed" => seed = args.next().and_then(|n| n.parse().ok()).expect("--seed S"),
            "--no-std" => std = false,
            "--details" => details = true,
            "--json" => json = true,
            _ => paths.push(PathBuf::from(arg)),
        }
    }

    let mut units = Vec::new();
    let mut files = Vec::new();
    for p in &paths {
        collect(p, &mut files);
    }
    files.sort();
    for file in files {
        if let Ok(text) = std::fs::read_to_string(&file) {
            units.push(Unit { origin: file.display().to_string(), text });
        }
    }
    for path in &snippets {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let list: Vec<Snippet> = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let name = path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().to_string();
        units.extend(
            list.into_iter()
                .enumerate()
                .map(|(i, s)| Unit { origin: format!("{name}#{i} ({})", s.origin), text: s.snippet }),
        );
    }

    let engine_state = engine(std);
    let config = ParseConfig::new();
    let mut originals = Summary::default();
    let mut mutated = Summary::default();
    let mut rng = Rng(seed);
    // From here on panics are counted, not printed. Installed only now, so that a bad
    // argument or snippet file above still reports its message.
    std::panic::set_hook(Box::new(|_| {}));
    for unit in &units {
        compare(&engine_state, &config, unit, &mut originals, details);
        for i in 0..mutants {
            let mutant = Unit { origin: format!("{} mutant {i}", unit.origin), text: mutate(&unit.text, &mut rng) };
            compare(&engine_state, &config, &mutant, &mut mutated, details);
        }
    }

    let rows = [("originals", &originals), ("mutants", &mutated)];
    if json {
        let fields = |s: &Summary| {
            format!(
                "{{\"total\":{},\"agree\":{},\"ours_rejects\":{},\"nu_syntax_rejects\":{},\"nu_semantic_rejects\":{},\"panics\":{}}}",
                s.total, s.agree, s.ours_rejects, s.nu_syntax_rejects, s.nu_semantic_rejects, s.panics
            )
        };
        println!("{{\"originals\":{},\"mutants\":{}}}", fields(&originals), fields(&mutated));
    } else {
        println!(
            "{:<10} {:>8} {:>8} {:>13} {:>18} {:>20} {:>7}",
            "set", "total", "agree", "ours_rejects", "nu_syntax_rejects", "nu_semantic_rejects", "panics"
        );
        for (name, s) in rows {
            if s.total > 0 {
                println!(
                    "{name:<10} {:>8} {:>8} {:>13} {:>18} {:>20} {:>7}",
                    s.total, s.agree, s.ours_rejects, s.nu_syntax_rejects, s.nu_semantic_rejects, s.panics
                );
            }
        }
    }
    let bad = rows.iter().any(|(_, s)| s.ours_rejects + s.nu_syntax_rejects + s.panics > 0);
    std::process::exit(if bad { 1 } else { 0 });
}
