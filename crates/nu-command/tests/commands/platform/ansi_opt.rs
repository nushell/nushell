use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

/// Two resets, red, `hi` and a reset: the second reset does nothing, and the first and the red
/// can be one sequence.
const REDUNDANT: &str = "$'(ansi reset)(ansi reset)(ansi red)hi(ansi reset)'";

#[rstest]
#[case::default(REDUNDANT, "", "\x1b[;31mhi\x1b[m")]
#[case::compat(REDUNDANT, "--compat", "\x1b[0;31mhi\x1b[m")]
#[case::rep("'────────'", "--rep", "─\x1b[7b")]
// Starting with a reset makes the state known, so the spaces need no reset of their own.
#[case::visual(
    r#""\e[0m\e[31mfoo \e[0m \e[31mbar\e[0m""#,
    "--visual",
    "\x1b[;31mfoo  bar\x1b[m"
)]
#[case::visual_rep(
    r#""\e[0m\e[31m──── \e[0m \e[31m────\e[0m""#,
    "--visual --rep",
    "\x1b[;31m─\x1b[3b  ─\x1b[3b\x1b[m"
)]
fn shortens_strings(#[case] input: &str, #[case] flags: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{input} | ansi opt {flags}"))
        .expect_value_eq(expected)
}

#[test]
fn shortens_binary() -> Result {
    test()
        .run("0x[1b 5b 30 6d 1b 5b 30 6d 41] | ansi opt")
        .expect_value_eq(Value::test_binary(b"\x1b[mA".to_vec()))
}

/// A stream with a sequence split between two chunks comes out shortened. `str join` streams
/// each list item as its own chunk, but the chunks can also arrive together, so either result
/// is right; the unit tests in `opt.rs` pin the chunk by chunk output.
#[test]
fn shortens_a_stream_with_a_sequence_split_between_chunks() -> Result {
    let output: String = test().run(r#"["\e[0m\e[" "0m\e[31mhi"] | str join | ansi opt"#)?;
    let (chunked, whole) = ("\x1b[m\x1b[31mhi", "\x1b[;31mhi");
    assert!([chunked, whole].contains(&output.as_str()), "{output:?}");
    Ok(())
}

/// An error from upstream comes out of the stream as itself, not as an I/O error.
#[test]
fn stream_passes_upstream_errors_through() -> Result {
    let code = r#"
        try {
            1..3
            | each {|i| if $i == 2 { error make {msg: boom} } else { $"($i)" } }
            | str join
            | ansi opt
        } catch {|err| $err.msg }
    "#;
    test().run(code).expect_value_eq("boom")
}

#[rstest]
#[case::one_line(REDUNDANT, r"\e[;31mhi\e[m")]
// Each line feed shows at the end of its line, and the last one starts no empty line.
#[case::lines(r#""\e[0m\e[0ma\nb\n""#, "\\e[ma\\n\nb\\n")]
fn show_makes_escapes_visible(#[case] input: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("{input} | ansi opt --show"))
        .expect_value_eq(expected)
}

#[test]
fn stats_reports_sizes() -> Result {
    test()
        .run(format!(
            "{REDUNDANT} | ansi opt --stats | reject percent breakdown changes"
        ))
        .expect_value_eq(test_record! {
            "input" => Value::test_filesize(19),
            "output" => Value::test_filesize(11),
            "saved" => Value::test_filesize(8),
            "sequences_in" => 4,
            "sequences_out" => 2,
            "fell_back" => false,
        })
}

#[test]
fn stats_groups_changes() -> Result {
    test()
        .run(format!("{REDUNDANT} | ansi opt --stats | get changes"))
        .expect_value_eq(test_table![
            ["saved", "count", "from", "to"];
            [Value::test_filesize(7), 1, r"\e[Nm\e[Nm\e[Nm", r"\e[;Nm"],
            [Value::test_filesize(1), 1, r"\e[Nm", r"\e[m"],
        ])
}

#[test]
fn explain_lists_every_sequence() -> Result {
    // The title is kept as it was, and still listed.
    test()
        .run(r#""\e]0;title\a\e[31ma\e[0m" | ansi opt --explain"#)
        .expect_value_eq(test_table![
            ["sequence", "meaning", "outcome", "output", "why"];
            [r"\e]0;title\x07", r#"title and icon: "title""#, "kept", (), ()],
            [r"\e[31m", "fg red", "folded", r"\e[31m", ()],
            [r"\e[0m", "reset", "folded", r"\e[m", ()],
        ])
}

#[test]
fn diff_lists_each_change() -> Result {
    test()
        .run(format!("{REDUNDANT} | ansi opt --diff"))
        .expect_value_eq(test_table![
            ["from", "to", "saved"];
            [r"\e[0m \e[0m \e[31m", r"\e[;31m", Value::test_filesize(7)],
            [r"\e[0m", r"\e[m", Value::test_filesize(1)],
        ])
}

#[test]
fn hotspot_totals_each_line() -> Result {
    let code = r#"
        $env.config.use_ansi_coloring = false
        $"(ansi red)a(ansi reset)\n(ansi red)b(ansi reset)" | ansi opt --hotspot
    "#;
    test().run(code).expect_value_eq(test_table![
        ["line", "escapes_in", "escapes_out", "text"];
        [1, Value::test_filesize(9), Value::test_filesize(8), "a"],
        [2, Value::test_filesize(9), Value::test_filesize(8), "b"],
    ])
}

/// A table shows the columns in this order and cuts off the last ones when it is too narrow.
#[rstest]
#[case::stats(
    "--stats",
    &[
        "input", "output", "saved", "percent", "sequences_in", "sequences_out", "fell_back",
        "breakdown", "changes",
    ]
)]
#[case::stats_breakdown(
    "--stats | get breakdown",
    &["category", "input", "output", "sequences_in", "sequences_out"]
)]
#[case::stats_changes("--stats | get changes", &["saved", "count", "from", "to"])]
#[case::diff("--diff", &["from", "to", "saved"])]
#[case::explain(
    "--explain",
    &["sequence", "meaning", "outcome", "output", "why"]
)]
#[case::hotspot("--hotspot", &["line", "escapes_in", "escapes_out", "text"])]
fn view_columns_are_in_reading_order(#[case] view: &str, #[case] columns: &[&str]) -> Result {
    test()
        .run(format!("{REDUNDANT} | ansi opt {view} | columns"))
        .expect_value_eq(columns.to_vec())
}

#[test]
fn html_returns_a_report() -> Result {
    let html: String = test().run(format!("{REDUNDANT} | ansi opt --html"))?;
    assert_contains("<title>ansiopt report: input</title>", html);
    Ok(())
}

/// The error names the flags in the order they were typed.
#[test]
fn views_are_exclusive() -> Result {
    let err = test()
        .run(format!("{REDUNDANT} | ansi opt --diff --stats"))
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::IncompatibleParameters { left_message, right_message, .. }
            if left_message == "--diff returns one view" && right_message == "--stats returns another"
    );
    Ok(())
}
