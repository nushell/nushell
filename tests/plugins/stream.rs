use nu_test_support::nu_with_plugins;
use pretty_assertions::assert_eq;

#[test]
fn seq_produces_stream() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "example seq 1 5 | describe"
    );

    assert_eq!(actual.out, "list<int> (stream)");
}

#[test]
fn seq_describe_no_collect_succeeds_without_error() {
    // This tests to ensure that there's no error if the stream is suddenly closed
    // Test several times, because this can cause different errors depending on what is written
    // when the engine stops running, especially if there's partial output
    for _ in 0..10 {
        let actual = nu_with_plugins!(
            cwd: "tests/fixtures/formats",
            plugin: ("nu_plugin_example"),
            "example seq 1 5 | describe --no-collect"
        );

        assert_eq!(actual.out, "stream");
        assert_eq!(actual.err, "");
    }
}

#[test]
fn seq_stream_collects_to_correct_list() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "example seq 1 5 | to json --raw"
    );

    assert_eq!(actual.out, "[1,2,3,4,5]");

    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "example seq 1 0 | to json --raw"
    );

    assert_eq!(actual.out, "[]");
}

#[test]
fn seq_big_stream() {
    // Testing big streams helps to ensure there are no deadlocking bugs
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "example seq 1 100000 | length"
    );

    assert_eq!(actual.out, "100000");
}

#[test]
fn sum_accepts_list_of_int() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "[1 2 3] | example sum"
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn sum_accepts_list_of_float() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "[1.0 2.0 3.5] | example sum"
    );

    assert_eq!(actual.out, "6.5");
}

#[test]
fn sum_accepts_stream_of_int() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "seq 1 5 | example sum"
    );

    assert_eq!(actual.out, "15");
}

#[test]
fn sum_accepts_stream_of_float() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "seq 1 5 | into float | example sum"
    );

    assert_eq!(actual.out, "15");
}

#[test]
fn sum_big_stream() {
    // Testing big streams helps to ensure there are no deadlocking bugs
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "seq 1 100000 | example sum"
    );

    assert_eq!(actual.out, "5000050000");
}

#[test]
fn collect_external_accepts_list_of_string() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "[a b] | example collect-external"
    );

    assert_eq!(actual.out, "ab");
}

#[test]
fn collect_external_accepts_list_of_binary() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "[0x[41] 0x[42]] | example collect-external"
    );

    assert_eq!(actual.out, "AB");
}

#[test]
fn collect_external_produces_raw_input() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "[a b c] | example collect-external | describe"
    );

    assert_eq!(actual.out, "raw input");
}

#[test]
fn collect_external_big_stream() {
    // This in particular helps to ensure that a big stream can be both read and written at the same
    // time without deadlocking
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        r#"(
            seq 1 10000 |
                to text |
                each { into string } |
                example collect-external |
                lines |
                length
        )"#
    );

    assert_eq!(actual.out, "10000");
}

#[test]
fn for_each_prints_on_stderr() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "[a b c] | example for-each { $in }"
    );

    assert_eq!(actual.err, "a\nb\nc\n");
}

#[test]
fn generate_sequence() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_example"),
        "example generate 0 { |i| if $i <= 10 { {out: $i, next: ($i + 2)} } } | to json --raw"
    );

    assert_eq!(actual.out, "[0,2,4,6,8,10]");
}
