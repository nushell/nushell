use iai_callgrind::{
    binary_benchmark_group, main, Arg, BinaryBenchmarkConfig, BinaryBenchmarkGroup, Fixtures, Run,
};

// Callgrid is a benchmarking tool that simulate the execution of a program on a virtual machine.
// It is useful to measure the performance of a program in a controlled environment.
// See https://github.com/iai-callgrind/iai-callgrind?tab=readme-ov-file#installation for installation instructions.
// You can run this benchmark suit by running `cargo bench --bench iai_benchmarks`.

fn my_setup() {}

binary_benchmark_group!(
    name = my_exe_group;
    setup = my_setup;
    // This directory will be copied into the root of the sandbox (as `fixtures`)
    config = BinaryBenchmarkConfig::default().fixtures(Fixtures::new("benches/fixtures"));
    benchmark =
        |"nu", group: &mut BinaryBenchmarkGroup| {
            setup_my_exe_group(group)
    }
);

fn setup_my_exe_group(group: &mut BinaryBenchmarkGroup) {
    group
        .bench(Run::with_arg(Arg::new(
            "standard startup",
            ["-c", "'exit'"],
        )))
        .bench(Run::with_arg(Arg::new(
            "clean startup",
            ["--no-std-lib", "--no-history", "-n", "-c", "'exit'"],
        )))
        .bench(Run::with_arg(Arg::new(
            "for loop",
            [
                "--no-std-lib",
                "--no-history",
                "-n",
                "-c",
                "'(for x in 1..10000 { echo $x }) | ignore'",
            ],
        )))
        .bench(Run::with_arg(Arg::new(
            "open json",
            [
                "--no-std-lib",
                "--no-history",
                "-n",
                "-c",
                "'open fixtures/json_example.json'",
            ],
        )))
        .bench(Run::with_arg(Arg::new(
            "math",
            ["--no-std-lib", "--no-history", "-n", "fixtures/math.nu"],
        )));
}

main!(binary_benchmark_groups = my_exe_group);
