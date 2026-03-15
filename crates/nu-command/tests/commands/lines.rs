use nu_test_support::nu;

#[test]
fn lines() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml -r
        | lines
        | skip while {|it| $it != "[dependencies]" }
        | skip 1
        | first
        | split column "="
        | get column0.0
        | str trim
    "#);

    assert_eq!(actual.out, "rustyline");
}

#[test]
fn lines_proper_buffering() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open lines_test.txt -r
        | lines
        | str length
        | to json -r
    ");

    assert_eq!(actual.out, "[8193,3]");
}

#[test]
fn lines_multi_value_split() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample-simple.json
        | get first second
        | lines
        | length
    ");

    assert_eq!(actual.out, "6");
}

/// test whether this handles CRLF and LF in the same input
#[test]
fn lines_mixed_line_endings() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        "foo\nbar\r\nquux" | lines | length
    "#);

    assert_eq!(actual.out, "3");
}

#[cfg(not(windows))]
#[test]
fn lines_on_error() {
    let actual = nu!("open . | lines");

    assert!(actual.err.contains("Is a directory"));
}

#[test]
fn lines_handles_non_utf8_bytes() {
    // Create a file with invalid UTF-8 bytes and verify `lines` doesn't error
    let actual = nu!(r#"
        0x[68 65 6C 6C 6F 0A 77 6F 72 6C 64 FF FE 0A 66 6F 6F]
        | save --force /tmp/nu_test_non_utf8.txt
        ; open --raw /tmp/nu_test_non_utf8.txt | lines | length
    "#);

    assert_eq!(actual.out, "3");
    assert!(actual.err.is_empty() || !actual.err.contains("Non-UTF8"));
}
