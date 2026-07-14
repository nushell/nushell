use nu_test_support::prelude::*;
use rstest::rstest;

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn seq_produces_stream() -> Result {
    test()
        .run("example seq 1 5 | describe")
        .expect_value_eq("list<int> (stream)")
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn seq_describe_no_collect_succeeds_without_error() -> Result {
    // This tests to ensure that there's no error if the stream is suddenly closed
    // Test several times, because this can cause different errors depending on what is written
    // when the engine stops running, especially if there's partial output
    for _ in 0..10 {
        test()
            .run("example seq 1 5 | describe --no-collect")
            .expect_value_eq("stream")?;
    }

    Ok(())
}

#[rstest]
#[case("example seq 1 5 | to json --raw", "[1,2,3,4,5]")]
#[case("example seq 1 0 | to json --raw", "[]")]
#[nu_test_support::test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn seq_stream_collects_to_correct_list(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn seq_big_stream() -> Result {
    test()
        .run("example seq 1 100_000 | length")
        .expect_value_eq(100_000)
}

#[rstest]
#[case::list_of_ints("[1 2 3]", 6)]
#[case::list_of_floats("[1.0 2.0 3.5]", 6.5)]
#[case::stream_of_ints("seq 1 5", 15)]
#[case::stream_of_floats("seq 1 5 | into float", 15.0)]
#[case::big_stream("seq 1 100_000", 5_000_050_000i64)]
#[nu_test_support::test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn sum_accepts(#[case] input: &str, #[case] expected: impl IntoValue) -> Result {
    test()
        .run(format!("{input} | example sum"))
        .expect_value_eq(expected)
}

#[rstest]
#[case::list_of_strings("[a b]", "ab")]
#[case::list_of_binary("[0x[41] 0x[42]]", "AB")]
#[nu_test_support::test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn collect_bytes(#[case] input: &str, #[case] expected: impl IntoValue) -> Result {
    test()
        .run(format!("{input} | example collect-bytes"))
        .expect_value_eq(expected)
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn collect_bytes_produces_byte_stream() -> Result {
    test()
        .run("[a b c] | example collect-bytes | describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn collect_bytes_big_stream() -> Result {
    let code = "
        seq 1 10_000
        | each {|i| ($i | into string) ++ (char newline) }
        | example collect-bytes
        | lines
        | length
    ";

    test().run(code).expect_value_eq(10_000)
}

#[test]
#[deps(NU, NU_PLUGIN_EXAMPLE)]
fn for_each_prints_on_stderr() -> Result {
    let code = format!(
        r#"nu -n --plugins {} -c "{}" | complete | get stderr"#,
        NU_PLUGIN_EXAMPLE.path().display(),
        "[a b c] | example for-each { $in }"
    );

    test().run(code).expect_value_eq("a\nb\nc\n")
}

#[test]
#[deps(NU_PLUGIN_EXAMPLE)]
fn generate_sequence() -> Result {
    let code = "
        example generate 0 {|i| if $i <= 10 { {out: $i, next: ($i + 2)} }}
        | to json --raw
    ";

    test().run(code).expect_value_eq("[0,2,4,6,8,10]")
}

#[rstest]
#[timeout(std::time::Duration::from_secs(6))]
#[nu_test_support::test]
#[serial]
#[deps(NU_PLUGIN_EXAMPLE)]
fn echo_interactivity_on_slow_pipelines() -> Result {
    // This test works by putting 0 on the upstream immediately, followed by 1 after 10 seconds.
    // If values aren't streamed to the plugin as they become available, `example echo` won't emit
    // anything until both 0 and 1 are available. The desired behavior is that `example echo` gets
    // the 0 immediately, which is consumed by `first`, allowing the pipeline to terminate early.
    let code = "
        [1]
        | each {|n| sleep 10sec; $n }
        | prepend 0
        | example echo
        | first
    ";

    test().run(code).expect_value_eq(0)
}
