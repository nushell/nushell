# run a piece of `nushell` code multiple times and measure the time of execution.
#
# this command returns a benchmark report of the following form:
#
# > **Note**
# > `std bench --pretty` will return a `string`.
@example "measure the performance of simple addition" { bench { 1 + 2 } -n 10 } --result {
    mean: (4µs + 956ns)
    std: (4µs + 831ns)
    times: [
        (19µs + 402ns)
        ( 4µs + 322ns)
        ( 3µs + 352ns)
        ( 2µs + 966ns)
        ( 3µs        )
        ( 3µs +  86ns)
        ( 3µs +  84ns)
        ( 3µs + 604ns)
        ( 3µs +  98ns)
        ( 3µs + 653ns)
    ]
}
@example "get a pretty benchmark report" { bench { 1 + 2 } --pretty } --result "3µs 125ns +/- 2µs 408ns"
export def main [
    code: closure  # the piece of `nushell` code to measure the performance of
    --rounds (-n): int = 50  # the number of benchmark rounds (hopefully the more rounds the less variance)
    --verbose (-v) # be more verbose (namely prints the progress)
    --pretty # shows the results in human-readable format: "<mean> +/- <stddev>"
]: [
    nothing -> record<mean: duration, std: duration, times: list<duration>>
    nothing -> string
] {
    let times: list<duration> = (
        seq 1 $rounds | each {|i|
            if $verbose { print -n $"($i) / ($rounds)\r" }
            timeit { do $code }
        }
    )

    if $verbose { print $"($rounds) / ($rounds)" }

    let report = {
        mean: ($times | math avg)
        min: ($times | math min)
        max: ($times | math max)
        std: ($times | into int | into float | math stddev | into int | into duration)
        times: ($times)
    }

    if $pretty {
        $"($report.mean) +/- ($report.std)"
    } else {
        $report
    }
}
