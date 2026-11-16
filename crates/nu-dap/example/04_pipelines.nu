# Pipelines and the `$in` variable.
# `$in` refers to the pipeline input — the value piped into the current block.
# Step through to see how data flows from one command to the next.

# Basic $in usage: the pipeline input is available as $in
let result = "hello world" | str upcase
print $result

# $in in a block — when you use a bare block in a pipeline, $in is the input
let processed = 42 | { value: $in, doubled: ($in * 2) }
print $processed

# $in vs explicit closure parameter: these are equivalent
let nums = [1, 2, 3, 4, 5]

# Using $in (implicit input)
let tripled = $nums | each { $in * 3 }
print $"Tripled (via $in): ($tripled)"

# Using explicit parameter (same result)
let tripled2 = $nums | each { |n| $n * 3 }
print $"Tripled via explicit param: ($tripled2)"

# Multi-stage pipeline — step through to watch intermediate values
let words = "the quick brown fox jumps over the lazy dog"
    | split row " "
    | where { ($in | str length) > 3 }
    | each { $in | str capitalize }
    | str join ", "
print $words

# Pipeline with table data — watch the table shrink through filters
let files = [
    { name: "app.ts", size: 2400, modified: "2024-01-15" }
    { name: "readme.md", size: 800, modified: "2024-02-01" }
    { name: "config.json", size: 150, modified: "2024-01-20" }
    { name: "main.rs", size: 5600, modified: "2024-03-10" }
    { name: "test.py", size: 1200, modified: "2024-02-28" }
]

let large_files = $files
    | where { $in.size > 1000 }
    | sort-by size --reverse
    | each { |f| $"($f.name) - ($f.size) bytes" }
print $large_files

# $in at top level of a block expression
let tag = "v1.2.3" | split row "." | {
    major: ($in | get 0)
    minor: ($in | get 1)
    patch: ($in | get 2)
}
print $tag

# Using $in to build a record from pipeline input
let stats = [10, 20, 30, 40, 50] | {
    count: ($in | length)
    sum: ($in | math sum)
    avg: ($in | math avg)
}
print $stats
