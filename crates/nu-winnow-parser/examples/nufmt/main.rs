//! `nufmt`-style formatter example.
//!
//! ```text
//! cargo run --example nufmt -- file.nu            # print formatted source
//! cargo run --example nufmt -- --write file.nu    # format in place
//! cargo run --example nufmt -- --check dir/       # exit 1 if any file would change
//! cargo run --example nufmt -- --config nufmt.nuon file.nu
//! echo 'ls|where size>1kb' | cargo run --example nufmt
//! ```
//!
//! The config file is a NUON record (parsed by this crate) whose keys are the
//! fields of [`format::Options`]: `indent`, `indent_char` (`space`/`tab`),
//! `line_length`, `margin` (an int, or `null` to keep the source's blank
//! lines), `comment_spacing`, `keep_alignment`, `trim_trailing_whitespace`,
//! `indent_pipelines`, `strip_redundant_parens`, `expand_def_bodies`,
//! `expand_complex_records`, `compact_simple_closures`,
//! `unquote_match_patterns`. nufmt's `exclude` is accepted and ignored.

mod format;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nu_winnow_parser::LineIndex;
use nu_winnow_parser::ast::{Expr, RecordItem};

use format::{IndentChar, Note, Options, format_with_notes};

/// Apply a NUON config file to `options`.
fn load_config(path: &Path, options: &mut Options) -> Result<(), String> {
    let name = path.display().to_string();
    let src = std::fs::read_to_string(path).map_err(|e| format!("{name}: {e}"))?;
    let ast = nu_winnow_parser::parse(&src).map_err(|e| e.render(&src, Some(&name)))?;
    let items = match ast.block.pipelines.as_slice() {
        [p] => match p.elements.as_slice() {
            [e] => match &e.expr.expr {
                Expr::Record(items) => items,
                _ => return Err(format!("{name}: expected a record")),
            },
            _ => return Err(format!("{name}: expected a record")),
        },
        _ => return Err(format!("{name}: expected a single record")),
    };
    let usize_of = |n: i64| usize::try_from(n).ok();
    for item in items {
        let RecordItem::Pair { key, value, .. } = item else {
            return Err(format!("{name}: spread is not allowed in a config record"));
        };
        let Expr::String(k) = &key.expr else {
            return Err(format!("{name}: option names must be strings"));
        };
        let key = k.value.as_ref();
        let text = value.span.slice(&src);
        let bad = || format!("{name}: `{key}` cannot be `{text}`");
        match (key, &value.expr) {
            ("indent", Expr::Int(n)) => options.indent = usize_of(*n).ok_or_else(bad)?,
            ("indent_char", Expr::String(s)) => {
                options.indent_char = match s.value.as_ref() {
                    "space" => IndentChar::Space,
                    "tab" => IndentChar::Tab,
                    _ => return Err(bad()),
                }
            }
            ("line_length", Expr::Int(n)) => options.line_length = usize_of(*n).ok_or_else(bad)?,
            ("margin", Expr::Int(n)) => options.margin = Some(usize_of(*n).ok_or_else(bad)?),
            ("margin", Expr::Nothing) => options.margin = None,
            ("comment_spacing", Expr::Int(n)) => options.comment_spacing = usize_of(*n).ok_or_else(bad)?,
            ("keep_alignment", Expr::Bool(b)) => options.keep_alignment = *b,
            ("trim_trailing_whitespace", Expr::Bool(b)) => options.trim_trailing_whitespace = *b,
            ("indent_pipelines", Expr::Bool(b)) => options.indent_pipelines = *b,
            ("strip_redundant_parens", Expr::Bool(b)) => options.strip_redundant_parens = *b,
            ("expand_def_bodies", Expr::Bool(b)) => options.expand_def_bodies = *b,
            ("expand_complex_records", Expr::Bool(b)) => options.expand_complex_records = *b,
            ("compact_simple_closures", Expr::Bool(b)) => options.compact_simple_closures = *b,
            ("unquote_match_patterns", Expr::Bool(b)) => options.unquote_match_patterns = *b,
            ("exclude", _) => {}
            (
                "indent"
                | "indent_char"
                | "line_length"
                | "margin"
                | "comment_spacing"
                | "keep_alignment"
                | "trim_trailing_whitespace"
                | "indent_pipelines"
                | "strip_redundant_parens"
                | "expand_def_bodies"
                | "expand_complex_records"
                | "compact_simple_closures"
                | "unquote_match_patterns",
                _,
            ) => return Err(bad()),
            _ => return Err(format!("{name}: unknown option `{key}`")),
        }
    }
    Ok(())
}

/// Print the formatter's notes (rewrites beyond whitespace) to stderr.
fn report(notes: &[Note], src: &str, name: &str) {
    let index = LineIndex::new(src);
    for note in notes {
        let at = index.line_col(note.offset, src);
        eprintln!("{name}:{}:{}: note: {}", at.line, at.column, note.message);
    }
}

/// Every `.nu` file below `path`, recursively.
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

fn main() -> ExitCode {
    let mut write = false;
    let mut check = false;
    let mut paths = Vec::new();
    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--write" | "-w" => write = true,
            "--check" => check = true,
            "--config" => {
                let Some(file) = args.next() else {
                    eprintln!("error: --config needs a file");
                    return ExitCode::from(2);
                };
                if let Err(e) = load_config(Path::new(&file), &mut options) {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            }
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    if paths.is_empty() {
        let mut src = String::new();
        if std::io::stdin().read_to_string(&mut src).is_err() {
            eprintln!("error: stdin is not valid UTF-8");
            return ExitCode::FAILURE;
        }
        return match format_with_notes(&src, &options) {
            Ok((out, notes)) => {
                report(&notes, &src, "<stdin>");
                print!("{out}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprint!("{}", e.render(&src, Some("<stdin>")));
                ExitCode::FAILURE
            }
        };
    }
    // A directory contributes its `.nu` files; a file named on the command line
    // is formatted whatever its name (a `#!/usr/bin/env nu` script), and one that
    // cannot be read is an error, not a file left out.
    let mut files = Vec::new();
    for p in &paths {
        if p.is_dir() {
            collect(p, &mut files);
        } else {
            files.push(p.clone());
        }
    }
    files.sort();
    let mut status = ExitCode::SUCCESS;
    let mut changed = 0;
    for file in &files {
        let src = match std::fs::read_to_string(file) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", file.display());
                status = ExitCode::FAILURE;
                continue;
            }
        };
        match format_with_notes(&src, &options) {
            Ok((out, notes)) => {
                report(&notes, &src, &file.display().to_string());
                if check {
                    if out != src {
                        println!("would reformat {}", file.display());
                        changed += 1;
                    }
                } else if write {
                    if out != src {
                        std::fs::write(file, &out).expect("write");
                        println!("formatted {}", file.display());
                    }
                } else {
                    print!("{out}");
                }
            }
            Err(e) => {
                eprint!("{}", e.render(&src, Some(&file.display().to_string())));
                status = ExitCode::FAILURE;
            }
        }
    }
    if check && changed > 0 {
        println!("{changed} of {} files would be reformatted", files.len());
        return ExitCode::FAILURE;
    }
    status
}
