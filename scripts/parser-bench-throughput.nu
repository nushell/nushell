#!/usr/bin/env nu
# Parse parser benchmark output and report throughput (chars/sec and bytes/sec).
# Compatible with Nushell 0.112.2.

def unit-to-seconds [value: float, unit: string] {
    match $unit {
        "ns" => { $value / 1_000_000_000.0 }
        "us" => { $value / 1_000_000.0 }
        "µs" => { $value / 1_000_000.0 }
        "ms" => { $value / 1_000.0 }
        "s" => { $value }
        _ => { null }
    }
}

def normalize-time-unit [unit: string] {
    let normalized = ($unit | str downcase)

    match $normalized {
        "sec" | "s" | "second" | "seconds" => { "sec" }
        "milli" | "ms" | "millisecond" | "milliseconds" => { "milli" }
        "micro" | "us" | "µs" | "microsecond" | "microseconds" => { "micro" }
        "nano" | "ns" | "nanosecond" | "nanoseconds" => { "nano" }
        _ => { null }
    }
}

def selected-unit-to-seconds [unit: string] {
    match $unit {
        "sec" => { 1.0 }
        "milli" => { 0.001 }
        "micro" => { 0.000001 }
        "nano" => { 0.000000001 }
        _ => { null }
    }
}

def extract-sizes [name: string] {
    let capture = ($name | parse -r '_(?<bytes>[0-9]+)b_(?<chars>[0-9]+)c$')

    if ($capture | is-empty) {
        null
    } else {
        {
            bytes: ($capture.0.bytes | into int)
            chars: ($capture.0.chars | into int)
        }
    }
}

def main [
    log_file?: path   # output file produced by parser-bench-0.112.2-run.nu
    --name-prefix: string = "parser_"
    --time-unit: string = "micro"  # throughput time base: sec|milli|micro|nano
] {
    if $log_file == null {
        error make {
            msg: "Missing required argument: log_file"
            help: "Usage: parser-bench-throughput.nu <log-file>\n\nExample:\n  ./target/debug/nu scripts/parser-bench-throughput.nu target/parser-bench/benchmarks-parser__-20260519-125213.log"
        }
    }

    let normalized_time_unit = (normalize-time-unit $time_unit)
    if $normalized_time_unit == null {
        error make {
            msg: $"unsupported time unit: ($time_unit)"
            help: "supported values: sec, milli, micro, nano"
        }
    }

    let selected_unit_seconds = (selected-unit-to-seconds $normalized_time_unit)

    let rows = (
        open $log_file
        | lines
        | parse -r '^(?<name>parser_[A-Za-z0-9_]+)\s+\[.*?\.\.\.\s*(?<value>[0-9]+(?:\.[0-9]+)?)\s*(?<unit>ns|us|µs|ms|s)\s*\.\.\.'
        | where {|row| $row.name | str starts-with $name_prefix }
        | each {|row|
            let seconds = (unit-to-seconds ($row.value | into float) $row.unit)
            let sizes = (extract-sizes $row.name)
            let unit_time = if $seconds == null {
                null
            } else {
                $seconds / $selected_unit_seconds
            }

            if ($seconds == null or $sizes == null or $unit_time == null or $unit_time == 0.0) {
                null
            } else {
                {
                    benchmark: $row.name
                    seconds: ($seconds | into float)
                    chars: $sizes.chars
                    bytes: $sizes.bytes
                    time_unit: $normalized_time_unit
                    chars_per_unit: (($sizes.chars | into float) / $unit_time)
                    bytes_per_unit: (($sizes.bytes | into float) / $unit_time)
                }
            }
        }
        | compact
        | uniq-by benchmark
        | sort-by benchmark
    )

    if ($rows | is-empty) {
        error make {
            msg: "no parser benchmark rows were parsed"
            help: "ensure the log contains parser benchmark output lines with timing units"
        }
    }

    print $"Throughput summary \(derived from benchmark timing, unit: ($normalized_time_unit)\):"
    print ($rows
    | select benchmark chars bytes seconds time_unit chars_per_unit bytes_per_unit
    | table)

    print $"\nHeadline metric candidates \(chars/($normalized_time_unit)\):"
    print ($rows
    | sort-by chars_per_unit --reverse
    | first 5
    | select benchmark chars_per_unit
    | table)

    # print "\nFull table of all parsed benchmarks with throughput metrics:"
    # print $rows
    
    print $"\nBenchmark logfile: ($log_file)"
    open $log_file
}
