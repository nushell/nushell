#!/usr/bin/env nu
# Manual smoke benchmarks for mut-assign / concat / par-each opts.
#
# Run: nu benches/mut_assign_bench.nu
# Prefer cargo tango benches for branch-vs-main comparison:
#   cargo bench --bench benchmarks -- solo --filter 'mut_record_assign|stack_upsert|par_each_many|value_concat'
#
# Note: std/bench closures can't capture outer `mut` variables,
# so each benchmark declares `mut` inside the closure (setup included).

use std/bench *

let n = 10000
let large_record = {a: (1..$n | each {|i| $i | into string} | str join)}
let large_list = (1..$n | each {|i| {a: $i}})
let prebuilt = 1..$n | each {|i| $i}

print $"Large data size: ($n) elements"
print $"Record value length: ($large_record.a | str length) bytes"
print ""

# --- Record field assignment (optimized UpdateVarCellPath path) ---
print "=== Record field assignment ($r.a = 'x') ==="
bench --rounds 50 --warmup 5 --pretty { mut r = $large_record; $r.a = 'x' }

# --- List element assignment ---
print "=== List element assignment ($l.0 = {a: 999}) ==="
bench --rounds 50 --warmup 5 --pretty { mut l = $large_list; $l.0 = {a: 999} }

# --- Pipeline update (slow baseline for comparison) ---
print "=== Pipeline update ($r = ($r | update a { 'x' })) ==="
bench --rounds 50 --warmup 5 --pretty { mut r = $large_record; $r = ($r | update a { 'x' }) }

# --- Compound assignment ---
print "=== Compound assignment ($r.a += 1) ==="
bench --rounds 50 --warmup 5 --pretty { mut r = {a: $n}; $r.a += 1 }

# --- Concat with prebuilt lists (empty-side shortcuts) ---
print "=== Concat general ($prebuilt ++ $prebuilt) ==="
bench --rounds 50 --warmup 5 --pretty { $prebuilt ++ $prebuilt | ignore }

print "=== Concat empty LHS ([] ++ $prebuilt) ==="
bench --rounds 50 --warmup 5 --pretty { [] ++ $prebuilt | ignore }

print "=== Concat empty RHS ($prebuilt ++ []) ==="
bench --rounds 50 --warmup 5 --pretty { $prebuilt ++ [] | ignore }

# --- Many small par-each (global pool reuse) ---
print "=== Many small par-each (default pool) ==="
bench --rounds 20 --warmup 3 --pretty {
    for _ in 1..50 { (1..10) | par-each {|_| 1 } | ignore }
}
