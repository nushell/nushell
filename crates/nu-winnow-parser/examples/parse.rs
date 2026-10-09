//! Parse Nushell source and print the AST or a summary.
//!
//! ```text
//! cargo run --example parse -- script.nu          # tree dump
//! cargo run --example parse -- --check dir/       # parse every .nu file, report failures
//! cargo run --example parse -- --summary script.nu
//! echo 'ls | length' | cargo run --example parse   # from stdin
//! cargo run --features serde --example parse -- --json script.nu
//! cargo run --example parse -- --flat script.nu    # tab-separated (start, end, shape) rows
//! ```
//!
//! Set `NU_WINNOW_COMMANDS=path` to a file with one command name per line to
//! make additional (e.g. standard-library) multi-word commands known.
#![allow(
    clippy::disallowed_types,
    clippy::unwrap_used,
    reason = "a native command-line example; `Instant` is disallowed only for WASM"
)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use nu_winnow_parser::ast::{Ast, Expr, Visitor};
use nu_winnow_parser::{ParseConfig, parse_lenient, pretty};

#[derive(Default)]
struct Options {
    check: bool,
    summary: bool,
    json: bool,
    flat: bool,
    quiet: bool,
    paths: Vec<PathBuf>,
}

fn usage() -> ExitCode {
    eprintln!("usage: parse [--check] [--summary] [--json] [--quiet] [FILE|DIR ...]");
    eprintln!("  With no paths, reads Nushell source from stdin.");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut opts = Options::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--check" => opts.check = true,
            "--summary" => opts.summary = true,
            "--json" => opts.json = true,
            "--flat" => opts.flat = true,
            "--quiet" | "-q" => opts.quiet = true,
            "--help" | "-h" => return usage(),
            _ => opts.paths.push(PathBuf::from(arg)),
        }
    }
    let mut config = ParseConfig::new();
    if let Ok(path) = std::env::var("NU_WINNOW_COMMANDS")
        && let Ok(list) = std::fs::read_to_string(&path)
    {
        config = config.add_commands(list.lines().map(str::trim).filter(|l| !l.is_empty()));
    }
    if opts.paths.is_empty() {
        let mut source = String::new();
        if std::io::stdin().read_to_string(&mut source).is_err() {
            eprintln!("error: stdin is not valid UTF-8");
            return ExitCode::FAILURE;
        }
        return report(&source, "<stdin>", &config, &opts);
    }
    if opts.check {
        return check(&opts, &config);
    }
    let mut status = ExitCode::SUCCESS;
    for path in &opts.paths {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        };
        if report(&source, &path.display().to_string(), &config, &opts) == ExitCode::FAILURE {
            status = ExitCode::FAILURE;
        }
    }
    status
}

fn report(source: &str, name: &str, config: &ParseConfig, opts: &Options) -> ExitCode {
    let start = Instant::now();
    let (ast, diagnostics) = parse_lenient(source, config);
    let elapsed = start.elapsed();
    if opts.json {
        print_json(&ast);
    } else if opts.flat {
        print_flat(&ast);
    } else if opts.summary || opts.check {
        let stats = Stats::of(&ast);
        println!(
            "{name}: {} bytes, {} lines, {} pipelines, {} expressions, {} comments, {} errors ({elapsed:?})",
            source.len(),
            source.lines().count(),
            stats.pipelines,
            stats.exprs,
            ast.comments.len(),
            diagnostics.len()
        );
    } else {
        print!("{}", pretty::dump(&ast));
    }
    if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        let error = nu_winnow_parser::ParseError::new(diagnostics);
        eprint!("{}", error.render(source, Some(name)));
        ExitCode::FAILURE
    }
}

/// Print `start\tend\tshape` rows from `flatten`, followed by the outer spans of
/// signatures (`sig` for `def`/`extern`, `closure-sig` for closure parameters)
/// so that consumers can group the rows inside them.
fn print_flat(ast: &Ast<'_>) {
    use nu_winnow_parser::ast::{Expression, Signature};
    for (span, shape) in nu_winnow_parser::flatten::flatten(ast) {
        println!("{}\t{}\t{:?}", span.start, span.end, shape);
    }
    struct Sigs(Vec<(usize, usize, &'static str)>);
    impl<'a> Visitor<'a> for Sigs {
        fn visit_expression(&mut self, e: &Expression<'a>) {
            if let Expr::Closure(c) = &e.expr
                && let Some(sig) = &c.params
            {
                self.0.push((sig.span.start, sig.span.end, "closure-sig"));
            }
            nu_winnow_parser::ast::walk_expression(self, e);
        }
        fn visit_signature(&mut self, sig: &Signature<'a>) {
            if !self.0.iter().any(|(s, _, _)| *s == sig.span.start) {
                self.0.push((sig.span.start, sig.span.end, "sig"));
            }
            nu_winnow_parser::ast::walk_signature(self, sig);
        }
    }
    let mut sigs = Sigs(Vec::new());
    sigs.visit_block(&ast.block);
    for (start, end, kind) in sigs.0 {
        println!("{start}\t{end}\t{kind}");
    }
}

#[cfg(feature = "serde")]
fn print_json(ast: &Ast<'_>) {
    println!("{}", serde_json::to_string_pretty(ast).expect("serializable"));
}

#[cfg(not(feature = "serde"))]
fn print_json(_ast: &Ast<'_>) {
    eprintln!("error: --json requires building with `--features serde`");
}

/// Walk every `.nu` file under the given paths and report parse failures.
fn check(opts: &Options, config: &ParseConfig) -> ExitCode {
    let mut files = Vec::new();
    for path in &opts.paths {
        collect(path, &mut files);
    }
    files.sort();
    let mut failed = 0usize;
    let mut total_bytes = 0usize;
    let mut total_errors = 0usize;
    let start = Instant::now();
    for file in &files {
        let Ok(source) = std::fs::read_to_string(file) else { continue };
        total_bytes += source.len();
        let (_, diagnostics) = parse_lenient(&source, config);
        if !diagnostics.is_empty() {
            failed += 1;
            total_errors += diagnostics.len();
            if !opts.quiet {
                let first = &diagnostics[0];
                let pos = nu_winnow_parser::LineIndex::new(&source).line_col(first.span.start, &source);
                let ctx = first.context.first().map(|c| format!(" (in {c})")).unwrap_or_default();
                println!("{}:{pos}: {}{ctx}", file.display(), first.kind);
            }
        }
    }
    let elapsed = start.elapsed();
    let ok = files.len() - failed;
    println!(
        "{ok}/{} files parsed cleanly ({failed} with errors, {total_errors} diagnostics); {} bytes in {elapsed:?} ({:.1} MB/s)",
        files.len(),
        total_bytes,
        total_bytes as f64 / 1e6 / elapsed.as_secs_f64().max(1e-9)
    );
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
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

#[derive(Default)]
struct Stats {
    pipelines: usize,
    exprs: usize,
}

impl Stats {
    fn of(ast: &Ast<'_>) -> Self {
        let mut s = Stats::default();
        s.visit_block(&ast.block);
        s
    }
}

impl<'a> Visitor<'a> for Stats {
    fn visit_pipeline(&mut self, pipeline: &nu_winnow_parser::ast::Pipeline<'a>) {
        self.pipelines += 1;
        nu_winnow_parser::ast::walk_pipeline(self, pipeline);
    }

    fn visit_expression(&mut self, expr: &nu_winnow_parser::ast::Expression<'a>) {
        if !matches!(expr.expr, Expr::Garbage) {
            self.exprs += 1;
        }
        nu_winnow_parser::ast::walk_expression(self, expr);
    }
}
