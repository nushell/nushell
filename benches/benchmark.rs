use std::hint::black_box;
use tango_bench::{benchmark_fn, tango_benchmarks, tango_main, IntoBenchmarks};

pub fn factorial(mut n: usize) -> usize {
    let mut result = 1usize;
    while n > 0 {
        result = result.wrapping_mul(black_box(n));
        n -= 1;
    }
    result
}

fn factorial_benchmarks() -> impl IntoBenchmarks {
    [benchmark_fn("factorial", |b| b.iter(|| factorial(500)))]
}

tango_benchmarks!(factorial_benchmarks());
tango_main!();
