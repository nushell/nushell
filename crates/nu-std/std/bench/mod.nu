# run a piece of `nushell` code multiple times and measure the time of execution.
#
# this command returns a benchmark report of the following form:
# ```
# record<
#   mean: duration
#   std: duration
#   times: list<duration>
# >
# ```
#
# > **Note**
# > `std bench --pretty` will return a `string`.
#
# # Examples
#     measure the performance of simple addition
#     > std bench { 1 + 2 } -n 10 | table -e
#     ╭───────┬────────────────────╮
#     │ mean  │ 4µs 956ns          │
#     │ std   │ 4µs 831ns          │
#     │       │ ╭───┬────────────╮ │
#     │ times │ │ 0 │ 19µs 402ns │ │
#     │       │ │ 1 │  4µs 322ns │ │
#     │       │ │ 2 │  3µs 352ns │ │
#     │       │ │ 3 │  2µs 966ns │ │
#     │       │ │ 4 │        3µs │ │
#     │       │ │ 5 │   3µs 86ns │ │
#     │       │ │ 6 │   3µs 84ns │ │
#     │       │ │ 7 │  3µs 604ns │ │
#     │       │ │ 8 │   3µs 98ns │ │
#     │       │ │ 9 │  3µs 653ns │ │
#     │       │ ╰───┴────────────╯ │
#     ╰───────┴────────────────────╯
#
#     get a pretty benchmark report
#     > std bench { 1 + 2 } --pretty
#     3µs 125ns +/- 2µs 408ns
export def main [
    code: closure  # the piece of `nushell` code to measure the performance of
    --rounds (-n): int = 50  # the number of benchmark rounds (hopefully the more rounds the less variance)
    --verbose (-v) # be more verbose (namely prints the progress)
    --pretty # shows the results in human-readable format: "<mean> +/- <stddev>"
] {
    let times = (
        seq 1 $rounds | each {|i|
            if $verbose { print -n $"($i) / ($rounds)\r" }
            timeit { do $code } | into int | into float
        }
    )

    if $verbose { print $"($rounds) / ($rounds)" }

    let report = {
        mean: ($times | math avg | from ns)
        min: ($times | math min | from ns)
        max: ($times | math max | from ns)
        std: ($times | math stddev | from ns)
        times: ($times | each { from ns })
    }

    if $pretty {
        $"($report.mean) +/- ($report.std)"
    } else {
        $report
    }
}

# convert an integer amount of nanoseconds to a real duration
def "from ns" [] {
    [$in "ns"] | str join | into duration
}