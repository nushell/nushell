//! Parse files with `nu-parser` from the local Nushell checkout and report
//! whether each one parses cleanly, the way `nu-check` does.
//!
//! ```text
//! cargo run --release --bin nu-parser-check -- [--no-std] [--quiet] FILE ...
//! ```
//!
//! Prints one line per file: `ok <file>` or `error <file>: <first error>`,
//! and exits with status 1 if any file had parse errors. The engine state has
//! every built-in command and (unless `--no-std`) the standard library, so
//! `use std/log` resolves exactly as it does in the `nu` binary.

use std::path::PathBuf;
use std::process::ExitCode;

use nu_protocol::engine::StateWorkingSet;
use nushell_harness::engine;

pub fn main() -> ExitCode {
    let mut std = true;
    let mut quiet = false;
    let mut paths = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--no-std" => std = false,
            "--quiet" | "-q" => quiet = true,
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let engine_state = engine(std);
    let mut failed = false;
    for path in &paths {
        let name = path.display().to_string();
        let src = match std::fs::read(path) {
            Ok(s) => s,
            Err(e) => {
                println!("error {name}: cannot read: {e}");
                failed = true;
                continue;
            }
        };
        let mut working_set = StateWorkingSet::new(&engine_state);
        let _block = nu_parser::parse(&mut working_set, Some(&name), &src, false);
        match working_set.parse_errors.first() {
            None => {
                if !quiet {
                    println!("ok {name}");
                }
            }
            Some(err) => {
                failed = true;
                let mut text = err.to_string();
                if let Some(detail) = std::error::Error::source(err) {
                    text = format!("{text}: {detail}");
                }
                println!("error {name}: {text}");
            }
        }
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
