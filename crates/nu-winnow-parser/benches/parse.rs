//! Throughput benchmarks: `cargo bench`.

#![allow(clippy::unwrap_used, reason = "benchmarks of inputs known to parse")]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use nu_winnow_parser::{ParseConfig, parse_lenient, parse_with};
use std::hint::black_box;

const KITCHEN_SINK: &str = include_str!("../tests/corpus/kitchen_sink.nu");
const STD_ITER: &str = include_str!("../tests/corpus/std_iter.nu");
const STD_ASSERT: &str = include_str!("../tests/corpus/std_assert.nu");

fn bench_parse(c: &mut Criterion) {
    let config = ParseConfig::new();
    let mut group = c.benchmark_group("parse");
    for (name, src) in [("kitchen_sink", KITCHEN_SINK), ("std_iter", STD_ITER), ("std_assert", STD_ASSERT)] {
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_function(name, |b| b.iter(|| parse_with(black_box(src), &config).unwrap()));
    }
    // A large synthetic file: the corpus repeated.
    let big: String = std::iter::repeat_n(format!("{STD_ITER}\n{STD_ASSERT}\n"), 20).collect();
    group.throughput(Throughput::Bytes(big.len() as u64));
    group.bench_function("large_file", |b| b.iter(|| parse_lenient(black_box(&big), &config)));
    group.finish();

    let mut group = c.benchmark_group("snippets");
    for (name, src) in [
        ("pipeline", "ls | where size > 1kb | sort-by modified | get name | first 10"),
        ("math", "(1 + 2) * 3 ** 2 - 4 / 2 mod 3 and not false or $x in [1 2 3]"),
        ("record", "{a: 1, b: [1 2 3], c: {d: \"x\", e: $\"y ($z)\"}, ...$rest}"),
        ("closure", "each {|x, y: int| $x + $y | into string }"),
        ("def", "def --env f [a: int, --flag(-f): string = \"x\", ...rest]: nothing -> string { $a }"),
    ] {
        group.throughput(Throughput::Bytes(src.len() as u64));
        group.bench_function(name, |b| b.iter(|| parse_with(black_box(src), &config).unwrap()));
    }
    group.finish();

    let mut group = c.benchmark_group("lexer");
    group.throughput(Throughput::Bytes(KITCHEN_SINK.len() as u64));
    group.bench_function("kitchen_sink", |b| {
        b.iter(|| {
            nu_winnow_parser::lex::lex(black_box(KITCHEN_SINK), 0, nu_winnow_parser::lex::LexOptions::BLOCK).unwrap()
        })
    });
    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
