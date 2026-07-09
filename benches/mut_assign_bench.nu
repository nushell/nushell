#!/usr/bin/env nu
# Benchmark: mut variable assignment performance
#
# Run: nu benches/mut_assign_bench.nu
# Run on both before and after optimization to compare.
#
# Note: std/bench closures can't capture outer `mut` variables,
# so each benchmark declares `mut` inside the closure.
# This means setup cost is included in the measurement.

use std/bench *

let n = 10000
let large_record = {a: (1..$n | each {|i| $i | into string} | str join)}
let large_list = (1..$n | each {|i| {a: $i}})

print $"Large data size: ($n) elements"
print $"Record value length: ($large_record.a | str length) bytes"
print ""

# --- Record field assignment ---
print "=== Record field assignment ($r.a = 'x') ==="
bench --rounds 50 --warmup 5 --pretty { mut r = $large_record; $r.a = 'x' }

# --- List element assignment ---
print "=== List element assignment ($l.0 = {a: 999}) ==="
bench --rounds 50 --warmup 5 --pretty { mut l = $large_list; $l.0 = {a: 999} }

# --- Pipeline update (for comparison) ---
print "=== Pipeline update ($r = ($r | update a { 'x' })) ==="
bench --rounds 50 --warmup 5 --pretty { mut r = $large_record; $r = ($r | update a { 'x' }) }

# --- Compound assignment ---
print "=== Compound assignment ($r.a += 1) ==="
bench --rounds 50 --warmup 5 --pretty { mut r = {a: $n}; $r.a += 1 }
