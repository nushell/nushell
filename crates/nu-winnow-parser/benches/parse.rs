//! Throughput benchmarks, run with tango as nushell's own: `cargo bench --bench parse -- solo`.
//! Each name ends with the input's size in bytes, from which a throughput follows.

#![allow(clippy::unwrap_used, reason = "benchmarks of inputs known to parse")]

use nu_winnow_parser::{
    ParseConfig,
    lex::{LexOptions, lex},
    parse_lenient, parse_with,
};
use std::hint::black_box;
use tango_bench::{IntoBenchmarks, benchmark_fn, tango_benchmarks};

const KITCHEN_SINK: &str = include_str!("../tests/corpus/kitchen_sink.nu");
const STD_ITER: &str = include_str!("../tests/corpus/std_iter.nu");
const STD_ASSERT: &str = include_str!("../tests/corpus/std_assert.nu");

/// `group_name_<bytes>b`: the name of a benchmark of `source`.
fn name(group: &str, name: &str, source: &str) -> String {
    format!("{group}_{name}_{}b", source.len())
}

/// Whole files: the corpus, and a large file made by repeating it.
fn files() -> impl IntoBenchmarks {
    let large: String = std::iter::repeat_n(format!("{STD_ITER}\n{STD_ASSERT}\n"), 20).collect();
    let mut benchmarks: Vec<_> = [("kitchen_sink", KITCHEN_SINK), ("std_iter", STD_ITER), ("std_assert", STD_ASSERT)]
        .into_iter()
        .map(|(file, source)| {
            benchmark_fn(name("parse", file, source), move |b| {
                let config = ParseConfig::new();
                b.iter(move || parse_with(black_box(source), &config).unwrap())
            })
        })
        .collect();
    benchmarks.push(benchmark_fn(name("parse", "large_file", &large), move |b| {
        let config = ParseConfig::new();
        let large = large.clone();
        // The tree borrows the source, so it is dropped here.
        b.iter(move || {
            black_box(parse_lenient(black_box(&large), &config));
        })
    }));
    benchmarks
}

/// Small programs, one construct each.
fn snippets() -> impl IntoBenchmarks {
    [
        ("pipeline", "ls | where size > 1kb | sort-by modified | get name | first 10"),
        ("math", "(1 + 2) * 3 ** 2 - 4 / 2 mod 3 and not false or $x in [1 2 3]"),
        ("record", "{a: 1, b: [1 2 3], c: {d: \"x\", e: $\"y ($z)\"}, ...$rest}"),
        ("closure", "each {|x, y: int| $x + $y | into string }"),
        ("def", "def --env f [a: int, --flag(-f): string = \"x\", ...rest]: nothing -> string { $a }"),
    ]
    .map(|(snippet, source)| {
        benchmark_fn(name("snippets", snippet, source), move |b| {
            let config = ParseConfig::new();
            b.iter(move || parse_with(black_box(source), &config).unwrap())
        })
    })
}

/// The lexer alone.
fn lexer() -> impl IntoBenchmarks {
    [benchmark_fn(name("lexer", "kitchen_sink", KITCHEN_SINK), |b| {
        b.iter(|| lex(black_box(KITCHEN_SINK), 0, LexOptions::BLOCK).unwrap())
    })]
}

tango_benchmarks!(files(), snippets(), lexer());
