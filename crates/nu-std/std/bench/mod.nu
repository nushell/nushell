# run a piece of `nushell` code multiple times and measure the time of execution.
#
# this command returns a benchmark report in the form of a table/record, or a string if using `--pretty`
#
# if multiple commands are passed, it will show a comparison of their runtimes.
@example "measure the performance of simple addition" { bench { 1 + 2 } } --result {
    mean: 2308ns,
    min: 2000ns,
    max: 8500ns,
    std: 895ns
}
@example "do 10 runs and show the time of each" { bench { 1 + 2 } -n 10 --verbose } --result {
    mean: 3170ns,
    min: 2200ns,
    max: 9800ns,
    std: 2228ns,
    times: [
        9800ns,
        3100ns,
        2800ns,
        2300ns,
        2500ns,
        2200ns,
        2300ns,
        2300ns,
        2200ns,
        2200ns
    ]
}
@example "get a pretty benchmark report" { bench { 1 + 2 } --pretty } --result "3µs 125ns +/- 2µs 408ns"
@example "compare multiple commands" { bench { 2 + 4 } { 2 ** 4 } } --result [
    [
        code,
        mean,
        min,
        max,
        std,
        ratio
    ];
    [
        "{ 2 + 4 }",
        2406ns,
        2100ns,
        9400ns,
        1012ns,
        1.02732707087959
    ],
    [
        "{ 2 ** 4 }",
        2342ns,
        2100ns,
        5300ns,
        610ns,
        1.0
    ]
]
@example "compare multiple commands with pretty report" { bench { 2 + 4 } { 2 ** 4 } --pretty } --result "
Benchmark 1: { 2 + 4 }
    2µs 494ns +/- 1µs 105ns
Benchmark 2: { 2 ** 4 }
    2µs 348ns +/- 565ns

{ 2 + 4 } ran
    1 times faster than { 2 ** 4 }"
@example "use --setup to compile before benchmarking" { bench { ./target/release/foo } --setup { cargo build --release } }
@example "use --warmup to fill the disk cache before benchmarking" { bench { fd } { jwalk . -k } -w 1 -n 10 }
export def main [
    ...commands: closure     # the piece(s) of `nushell` code to measure the performance of
    --rounds (-n): int = 50  # the number of benchmark rounds (hopefully the more rounds the less variance)
    --warmup (-w): int = 0   # the number of warmup rounds (not timed) to do before the benchmark, useful for filling the disk cache in I/O-heavy programs
    --setup (-s): closure    # command to run before all benchmarks
    --prepare: closure       # command to run before each benchmark (same as `--setup` if only doing one benchmark)
    --cleanup (-c): closure  # command to run after all benchmarks
    --conclude (-C): closure # command to run after each benchmark (same as `--cleanup` if only doing one benchmark)
    --ignore-errors (-i)     # ignore errors in the command
    --verbose (-v)           # show individual times (has no effect if used with `--pretty`)
    --progress (-P)          # prints the progress
    --pretty (-p)            # shows the results in human-readable format: "<mean> +/- <stddev>"
]: [
    nothing -> record<mean: duration, std: duration, times: list<duration>>
    nothing -> record<mean: duration, std: duration>
    nothing -> table<code: string, mean: duration, std: duration, ratio: float, times: list<duration>>
    nothing -> table<code: string, mean: duration, std: duration, ratio: float>
    nothing -> string
] {
    if $setup != null { do $setup | ignore }

    let results = (
        $commands | each {|code|

            if $prepare != null { do $prepare | ignore }

            seq 1 $warmup | each {|i|
                do --ignore-errors=$ignore_errors $code | ignore
            }

            let times: list<duration> = (
                seq 1 $rounds | each {|i|
                    if $progress { print -n $"($i) / ($rounds)\r" }
                    timeit { do --ignore-errors=$ignore_errors $code | ignore }
                }
            )

            if $progress { print $"($rounds) / ($rounds)" }

            if $cleanup != null { do $cleanup | ignore }

            {
                mean: ($times | math avg)
                min: ($times | math min)
                max: ($times | math max)
                std: ($times | into int | into float | math stddev | into int | into duration)
            }
            | if $verbose { merge { times: $times }} else {}
        }
    )

    if $conclude != null { do $conclude | ignore }

    # One benchmark
    if ($results | length) == 1 {
        let report = $results | first
        if $pretty {
            return $"($report.mean) +/- ($report.std)"
        } else {
            return $report
        }
    }

    # Multiple benchmarks
    let min_mean = $results | get mean | math min
    let results = (
        $commands
        | each { view source $in | nu-highlight }
        | wrap code
        | merge $results
        | insert ratio { $in.mean / $min_mean }
    )

    if $pretty {
        $results
        | enumerate
        | each {|x|
            let report = $x.item
            print $"Benchmark ($x.index + 1): ($report.code)\n\t($report.mean) +/- ($report.std)"
        }

        let results = $results | sort-by ratio

        print $"\n($results.0.code) ran"

        $results
        | skip
        | each {|report|
            print $"\t(ansi green)($report.ratio | math round -p 2)(ansi reset) times faster than ($report.code)"
        }

        ignore
    } else {
        $results
    }
}
