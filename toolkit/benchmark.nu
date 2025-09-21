# Benchmark a target revision (default: current branch) against a reference revision (default: main branch)
#
# Results are saved in a `./tango` directory
# Ensure you have `cargo-export` installed to generate separate artifacts for each branch.
export def benchmark-compare [
    target?: string     # which branch to compare (default: current branch)
    reference?: string  # the reference to compare against (default: main branch)
] {
    let reference = $reference | default "main"
    let current = git branch --show-current
    let target = $target | default $current

    print $'-- Benchmarking ($target) against ($reference)'

    let export_dir = $env.PWD | path join "tango"
    let ref_bin_dir = $export_dir | path join bin $reference
    let tgt_bin_dir = $export_dir | path join bin $target

    # benchmark the target revision
    print $'-- Running benchmarks for ($target)'
    git checkout $target
    ^cargo export $tgt_bin_dir -- bench

    # benchmark the comparison reference revision
    print $'-- Running benchmarks for ($reference)'
    git checkout $reference
    ^cargo export $ref_bin_dir -- bench

    # return back to the whatever revision before benchmarking
    print '-- Done'
    git checkout $current

    # report results
    let reference_bin = $ref_bin_dir | path join benchmarks
    let target_bin = $tgt_bin_dir | path join benchmarks
    ^$target_bin compare $reference_bin -o -s 50 --dump ($export_dir | path join samples)
}

# Benchmark the current branch and logs the result in `./tango/samples`
#
# Results are saved in a `./tango` directory
# Ensure you have `cargo-export` installed to generate separate artifacts for each branch.
export def benchmark-log [
    target?: string     # which branch to compare (default: current branch)
] {
    let current = git branch --show-current
    let target = $target | default $current
    print $'-- Benchmarking ($target)'

    let export_dir = $env.PWD | path join "tango"
    let bin_dir = ($export_dir | path join bin $target)

    # benchmark the target revision
    if $target != $current {
        git checkout $target
    }
    ^cargo export $bin_dir -- bench

    # return back to the whatever revision before benchmarking
    print '-- Done'
    if $target != $current {
        git checkout $current
    }

    # report results
    let bench_bin = ($bin_dir | path join benchmarks)
    ^$bench_bin compare -o -s 50 --dump ($export_dir | path join samples)
}
