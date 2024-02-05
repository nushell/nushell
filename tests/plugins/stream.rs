use nu_test_support::nu_with_plugins;
use pretty_assertions::assert_eq;

#[test]
fn seq_produces_stream() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "stream_example seq 1 5 | describe"
    );

    assert_eq!(actual.out, "list<int> (stream)");
}

#[test]
fn seq_describe_no_collect_succeeds_without_error() {
    // This tests to ensure that there's no error if the stream is suddenly closed
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "stream_example seq 1 5 | describe --no-collect"
    );

    assert_eq!(actual.out, "stream");
    assert_eq!(actual.err, "");
}

#[test]
fn seq_stream_collects_to_correct_list() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "stream_example seq 1 5 | to json --raw"
    );

    assert_eq!(actual.out, "[1,2,3,4,5]");

    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "stream_example seq 1 0 | to json --raw"
    );

    assert_eq!(actual.out, "[]");
}

#[test]
fn sum_accepts_list_of_int() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "[1 2 3] | stream_example sum"
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn sum_accepts_list_of_float() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "[1.0 2.0 3.5] | stream_example sum"
    );

    assert_eq!(actual.out, "6.5");
}

#[test]
fn sum_accepts_stream_of_int() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "seq 1 5 | stream_example sum"
    );

    assert_eq!(actual.out, "15");
}

#[test]
fn sum_accepts_stream_of_float() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "seq 1 5 | into float | stream_example sum"
    );

    assert_eq!(actual.out, "15");
}

#[test]
fn collect_external_accepts_list_of_string() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "[a b] | stream_example collect-external"
    );

    assert_eq!(actual.out, "ab");
}

#[test]
fn collect_external_accepts_list_of_binary() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "[0x[41] 0x[42]] | stream_example collect-external"
    );

    assert_eq!(actual.out, "AB");
}

#[test]
fn collect_external_produces_raw_input() {
    let actual = nu_with_plugins!(
        cwd: "tests/fixtures/formats",
        plugin: ("nu_plugin_stream_example"),
        "[a b c] | stream_example collect-external | describe"
    );

    assert_eq!(actual.out, "raw input");
}
