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
                | get source
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
        "open sample.bson | get root | nth 0 | get b"
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
            | nth 6
            | get b
            | get '$binary_subtype'
        "#
    ));

    assert_eq!(actual.out, "function");
}

// sample.db has the following format:
//
// ━━━┯━━━━━━━━━━━━┯━━━━━━━━━━━━━━
//  # │ table_name │ table_values
// ───┼────────────┼──────────────
//  0 │ strings    │ [6 items]
//  1 │ ints       │ [5 items]
//  2 │ floats     │ [4 items]
// ━━━┷━━━━━━━━━━━━┷━━━━━━━━━━━━━━
//
// In this case, this represents a sqlite database
// with three tables named `strings`, `ints`, and `floats`.
// The table_values represent the values for the tables:
//
// ━━━━┯━━━━━━━┯━━━━━━━━━━┯━━━━━━┯━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  #  │ x     │ y        │ z    │ f
// ────┼───────┼──────────┼──────┼──────────────────────────────────────────────────────────────────────
//   0 │ hello │ <binary> │      │
//   1 │ hello │ <binary> │      │
//   2 │ hello │ <binary> │      │
//   3 │ hello │ <binary> │      │
//   4 │ world │ <binary> │      │
//   5 │ world │ <binary> │      │
//   6 │       │          │    1 │
//   7 │       │          │   42 │
//   8 │       │          │  425 │
//   9 │       │          │ 4253 │
//  10 │       │          │      │
//  11 │       │          │      │                                                    3.400000000000000
//  12 │       │          │      │                                                    3.141592650000000
//  13 │       │          │      │                                                    23.00000000000000
//  14 │       │          │      │ this string that doesn't really belong here but sqlite is what it is
// ━━━━┷━━━━━━━┷━━━━━━━━━━┷━━━━━━┷━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// We can see here that each table has different columns. `strings` has `x` and `y`, while
// `ints` has just `z`, and `floats` has only the column `f`. This means, in general, when working
// with sqlite, one will want to select a single table, e.g.:
//
// open sample.db | nth 1 | get table_values
// ━━━┯━━━━━━
//  # │ z
// ───┼──────
//  0 │    1
//  1 │   42
//  2 │  425
//  3 │ 4253
//  4 │
// ━━━┷━━━━━━

#[cfg(feature = "sqlite")]
#[test]
fn parses_sqlite() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.db
            | get table_values
            | nth 2
            | get x
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
            | first 1
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
        "open jonathan.xml | get rss.children.channel.children | get item.children | get link.children.0"
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
        "open utf16.ini | rename info | get info | get IconIndex"
    );

    assert_eq!(actual.out, "-236")
}

#[test]
fn errors_if_file_not_found() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "open i_dont_exist.txt"
    );
    let expected = "Cannot find file";
    assert!(
        actual.err.contains(expected),
        "Error:\n{}\ndoes not contain{}",
        actual.err,
        expected
    );
}
