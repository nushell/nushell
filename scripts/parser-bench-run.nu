#!/usr/bin/env nu
# Run parser-focused benchmarks and persist the full output for throughput analysis.

def main [
    --filter: string = "parser_*"   # benchmark name glob filter (tango uses glob matching)
    --bench: string = "benchmarks"  # bench target name
    --out-dir: path = "target/parser-bench"  # directory where logs are written
] {
    mkdir $out_dir

    let timestamp = (date now | format date "%Y%m%d-%H%M%S")
    let safe_filter = ($filter | str replace -ar '[^A-Za-z0-9_-]' "_")
    let log_file = ($out_dir | path join $"($bench)-($safe_filter)-($timestamp).log")

    print $"Running parser benchmarks with filter: ($filter)"

    # Tango-bench: `cargo bench -- solo --filter <glob>` runs matching benchmarks.
    # The glob is passed to tango's --filter (-f) flag which uses glob_match internally.
    let result = (do { ^cargo bench --bench $bench -- solo --filter $filter } | complete)

    let combined_output = [$result.stdout $result.stderr]
        | where {|line| ($line | is-not-empty) }
        | str join (char nl)

    $combined_output | save --force $log_file

    if $result.exit_code == 0 {
        print $"Saved parser benchmark log to ($log_file)"
        $log_file
    } else {
        print $"Cargo bench failed with exit code ($result.exit_code)"
        print $"Output saved to ($log_file)"
        error make {
            msg: "cargo bench exited with non-zero code"
            help: $"Review output: ($log_file)"
        }
    }
}
