# A practical script: a mini project analyzer.
# Combines many nushell features in a realistic scenario.
# Good for end-to-end stepping through a complete workflow.

def count_lines [file: string] {
    open $file | lines | length
}

def analyze_extension [ext: string, files: list] {
    let matching = $files | where { $in.name | str ends-with $ext }
    let total_size = $matching | get size | math sum
    {
        extension: $ext
        count: ($matching | length)
        total_size: $total_size
    }
}

# --- Main logic ---

print "=== Project Analyzer ==="
print $"Working directory: ($env.PWD)\n"

# Gather all .nu files
let nu_files = ls *.nu
print $"Found ($nu_files | length) .nu files\n"

# Analyze each file
let analysis = $nu_files | each { |f|
    let lines = count_lines $f.name
    {
        name: $f.name
        size: $f.size
        lines: $lines
        bytes_per_line: (if $lines > 0 { ($f.size | into int) / $lines } else { 0 })
    }
}

# Sort and display
let by_lines = $analysis | sort-by lines --reverse
print "--- Files by line count ---"
print $by_lines

# Summary statistics
let total_lines = $analysis | get lines | math sum
let total_size = $analysis | get size | math sum
let avg_lines = $analysis | get lines | math avg

print $"\n--- Summary ---"
print $"Total files: ($analysis | length)"
print $"Total lines: ($total_lines)"
print $"Total size: ($total_size)"
print $"Average lines/file: ($avg_lines | math round --precision 1)"

# Find the largest and smallest
let largest = $by_lines | first
let smallest = $by_lines | last
print $"\nLargest: ($largest.name) \(($largest.lines) lines)"
print $"Smallest: ($smallest.name) \(($smallest.lines) lines)"
