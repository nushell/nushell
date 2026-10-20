//! Time `nu-parser` and `nu-winnow-parser` on the same files.
//!
//! ```text
//! cargo run --release --bin bench-vs-nu-parser -- [--iters N] [--std] FILE|DIR ...
//! ```
//!
//! Both parsers see the same bytes. `nu-parser` runs with the full built-in
//! command set (as the `nu` binary does) on a fresh `StateWorkingSet` per
//! iteration; its cost includes declaration resolution and type checking,
//! which `nu-winnow-parser` does not do, so the comparison measures the whole
//! "source to AST" step each engine performs rather than lexing alone.
//!
//! With `--std` the standard library is registered so that `use std/log`
//! resolves; `nu-parser` then parses the imported module sources inside the
//! timed region, which is part of what the shell does but not a
//! parser-to-parser comparison.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_winnow_parser::{ParseConfig, parse_lenient};
use nushell_harness::engine;

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

fn time_nu_parser(engine_state: &EngineState, name: &str, src: &[u8], iters: usize) -> (Duration, usize) {
    let mut total = Duration::ZERO;
    let mut errors = 0;
    for _ in 0..iters {
        let mut working_set = StateWorkingSet::new(engine_state);
        let start = Instant::now();
        let _block = nu_parser::parse(&mut working_set, Some(name), src, false);
        total += start.elapsed();
        errors = working_set.parse_errors.len();
    }
    (total, errors)
}

fn time_winnow(config: &ParseConfig, src: &str, iters: usize) -> (Duration, usize) {
    let mut total = Duration::ZERO;
    let mut errors = 0;
    for _ in 0..iters {
        let start = Instant::now();
        let (_ast, diagnostics) = parse_lenient(src, config);
        total += start.elapsed();
        errors = diagnostics.len();
    }
    (total, errors)
}

pub fn main() {
    let mut iters = 5usize;
    let mut std = false;
    let mut paths = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--iters" => match args.next().and_then(|n| n.parse::<usize>().ok()) {
                // A file's time is the mean over the iterations, so there must be one.
                Some(n) if n > 0 => iters = n,
                _ => {
                    eprintln!("error: --iters needs a number of iterations, at least 1");
                    std::process::exit(2);
                }
            },
            "--std" => std = true,
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let mut files = Vec::new();
    for p in &paths {
        collect(p, &mut files);
    }
    files.sort();
    let engine_state = engine(std);
    let config = ParseConfig::new();
    // Warm up both parsers so first-call costs are not counted.
    let warm = "ls | where size > 1kb | get name";
    let _ = time_nu_parser(&engine_state, "warm", warm.as_bytes(), 1);
    let _ = time_winnow(&config, warm, 1);

    println!(
        "{:<52} {:>8} {:>11} {:>11} {:>7} {:>5} {:>5}",
        "file", "bytes", "nu-parser", "winnow", "ratio", "nuErr", "wErr"
    );
    let (mut bytes_total, mut nu_total, mut w_total) = (0usize, Duration::ZERO, Duration::ZERO);
    for file in &files {
        let Ok(src) = std::fs::read_to_string(file) else { continue };
        let name = file.file_name().unwrap_or(file.as_os_str()).to_string_lossy();
        let (nu_t, nu_err) = time_nu_parser(&engine_state, &name, src.as_bytes(), iters);
        let (w_t, w_err) = time_winnow(&config, &src, iters);
        let nu_avg = nu_t / iters as u32;
        let w_avg = w_t / iters as u32;
        bytes_total += src.len();
        nu_total += nu_avg;
        w_total += w_avg;
        let ratio = nu_avg.as_secs_f64() / w_avg.as_secs_f64().max(1e-9);
        let short: String =
            file.display().to_string().chars().rev().take(50).collect::<Vec<_>>().into_iter().rev().collect();
        println!(
            "{short:<52} {:>8} {:>11} {:>11} {:>6.1}x {:>5} {:>5}",
            src.len(),
            fmt(nu_avg),
            fmt(w_avg),
            ratio,
            nu_err,
            w_err
        );
    }
    let ratio = nu_total.as_secs_f64() / w_total.as_secs_f64().max(1e-9);
    println!("{}", "-".repeat(104));
    println!(
        "{:<52} {:>8} {:>11} {:>11} {:>6.1}x",
        format!("TOTAL ({} files, {} iterations each)", files.len(), iters),
        bytes_total,
        fmt(nu_total),
        fmt(w_total),
        ratio
    );
    println!(
        "throughput: nu-parser {:.1} MB/s, nu-winnow-parser {:.1} MB/s",
        bytes_total as f64 / 1e6 / nu_total.as_secs_f64().max(1e-9),
        bytes_total as f64 / 1e6 / w_total.as_secs_f64().max(1e-9)
    );
}

fn fmt(d: Duration) -> String {
    let us = d.as_secs_f64() * 1e6;
    if us >= 1000.0 { format!("{:.2} ms", us / 1000.0) } else { format!("{us:.0} µs") }
}
