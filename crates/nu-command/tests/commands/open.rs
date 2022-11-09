use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn parses_csv() {
    Playground::setup("open_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu.zion.csv",
            r#"
                    author,lang,source
                    Jonathan Turner,Rust,New Zealand
                    Andres N. Robalino,Rust,Ecuador
                    Yehuda Katz,Rust,Estados Unidos
                "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open nu.zion.csv
                | where author == "Andres N. Robalino"
                | get source.0
            "#
        ));

        assert_eq!(actual.out, "Ecuador");
    })
}

// sample.bson has the following format:
// ━━━━━━━━━━┯━━━━━━━━━━━
//  _id      │ root
// ──────────┼───────────
//  [object] │ [9 items]
// ━━━━━━━━━━┷━━━━━━━━━━━
//
// the root value is:
// ━━━┯━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━┯━━━━━━━━━━┯━━━━━━━━━━
//  # │ _id               │ a                       │ b        │ c
// ───┼───────────────────┼─────────────────────────┼──────────┼──────────
//  0 │ [object]          │       1.000000000000000 │ hello    │ [2 items]
//  1 │ [object]          │       42.00000000000000 │ whel     │ hello
//  2 │ [object]          │ [object]                │          │
//  3 │ [object]          │                         │ [object] │
//  4 │ [object]          │                         │          │ [object]
//  5 │ [object]          │                         │          │ [object]
//  6 │ [object]          │ [object]                │ [object] │
//  7 │ [object]          │ <date value>            │ [object] │
//  8 │ 1.000000          │ <decimal value>         │ [object] │
//
// The decimal value is supposed to be π, but is currently wrong due to
// what appears to be an issue in the bson library that is under investigation.
//

#[cfg(feature = "bson")]
#[test]
fn parses_bson() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open sample.bson | get root | select 0 | get b"
    );

    assert_eq!(actual.out, "hello");
}

#[cfg(feature = "bson")]
#[test]
fn parses_more_bson_complexity() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.bson
            | get root
            | select 6
            | get b
            | get '$binary_subtype'
        "#
    ));

    assert_eq!(actual.out, "function");
}

// sample.db has the following format:
//
// ╭─────────┬────────────────╮
// │ strings │ [table 6 rows] │
// │ ints    │ [table 5 rows] │
// │ floats  │ [table 4 rows] │
// ╰─────────┴────────────────╯
//
// In this case, this represents a sqlite database
// with three tables named `strings`, `ints`, and `floats`.
//
// Each table has different columns. `strings` has `x` and `y`, while
// `ints` has just `z`, and `floats` has only the column `f`. In general, when working
// with sqlite, one will want to select a single table, e.g.:
//
// open sample.db | get ints
// ╭───┬──────╮
// │ # │  z   │
// ├───┼──────┤
// │ 0 │    1 │
// │ 1 │   42 │
// │ 2 │  425 │
// │ 3 │ 4253 │
// │ 4 │      │
// ╰───┴──────╯

#[cfg(feature = "database")]
#[test]
fn parses_sqlite() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | columns
            | length
        "#
    ));

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "database")]
#[test]
fn parses_sqlite_get_column_name() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get strings
            | get x.0
        "#
    ));

    assert_eq!(actual.out, "hello");
}

#[test]
fn parses_toml() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open cargo_sample.toml | get package.edition"
    );

    assert_eq!(actual.out, "2018");
}

#[test]
fn parses_tsv() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open caco3_plastics.tsv
            | first
            | get origin
        "#
    ));

    assert_eq!(actual.out, "SPAIN")
}

#[test]
fn parses_json() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sgml_description.json
            | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
        "#
    ));

    assert_eq!(actual.out, "markup")
}

#[test]
fn parses_xml() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open jonathan.xml | get rss.children.channel.children | get item.children | get link.children.0.3.3.0"
    );

    assert_eq!(
        actual.out,
        "http://www.jonathanturner.org/2015/10/off-to-new-adventures.html"
    )
}

#[test]
fn parses_ini() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open sample.ini | get SectionOne.integer"
    );

    assert_eq!(actual.out, "1234")
}

#[test]
fn parses_utf16_ini() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open ./utf16.ini --raw | decode utf-16 | from ini | rename info | get info | get IconIndex"
    );

    assert_eq!(actual.out, "-236")
}

#[cfg(feature = "dataframe")]
#[test]
fn parses_arrow_ipc() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open-df caco3_plastics.arrow
            | into nu
            | first
            | get origin
        "#
    ));

    assert_eq!(actual.out, "SPAIN")
}

#[test]
fn errors_if_file_not_found() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open i_dont_exist.txt"
    );
    // Common error code between unixes and Windows for "No such file or directory"
    //
    // This seems to be not directly affected by localization compared to the OS
    // provided error message
    let expected = "(os error 2)";

    assert!(
        actual.err.contains(expected),
        "Error:\n{}\ndoes not contain{}",
        actual.err,
        expected
    );
}

// FIXME: jt: I think `open` on a directory is confusing. We should make discuss this one a bit more
#[ignore]
#[test]
fn open_dir_is_ls() {
    Playground::setup("open_dir", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("yehuda.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("andres.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open .
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    })
}

#[test]
fn test_open_block_command() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
            def "from blockcommandparser" [] { lines | split column ",|," }
            let values = (open sample.blockcommandparser)
            echo ($values | get column1 | get 0)
            echo ($values | get column2 | get 0)
            echo ($values | get column1 | get 1)
            echo ($values | get column2 | get 1)
        "#
    );

    assert_eq!(actual.out, "abcd")
}

#[test]
fn open_ignore_ansi() {
    Playground::setup("open_test_ansi", |dirs, sandbox| {
        sandbox.with_files(vec![EmptyFile("nu.zion.txt")]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                ls | find nu.zion | get 0 | get name | open $in
            "#
        ));

        assert!(actual.err.is_empty());
    })
}

#[test]
fn open_no_parameter() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
            open
        "#
    );

    assert!(actual.err.contains("needs filename"));
}
