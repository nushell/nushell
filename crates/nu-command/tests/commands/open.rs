use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn parses_file_with_uppercase_extension() {
    Playground::setup("open_test_uppercase_extension", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            "nu.zion.JSON",
            r#"{
                "glossary": {
                    "GlossDiv": {
                        "GlossList": {
                            "GlossEntry": {
                                "ID": "SGML"
                            }
                        }
                    }
                }
            }"#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open nu.zion.JSON
                | get glossary.GlossDiv.GlossList.GlossEntry.ID
            "#
        ));

        assert_eq!(actual.out, "SGML");
    })
}

#[test]
fn parses_file_with_multiple_extensions() {
    Playground::setup("open_test_multiple_extensions", |dirs, sandbox| {
        sandbox.with_files(vec![
            FileWithContent("file.tar.gz", "this is a tar.gz file"),
            FileWithContent("file.tar.xz", "this is a tar.xz file"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                hide "from tar.gz" ;
                hide "from gz" ;

                def "from tar.gz" [] { 'opened tar.gz' } ;
                def "from gz" [] { 'opened gz' } ;
                open file.tar.gz
            "#
        ));

        assert_eq!(actual.out, "opened tar.gz");

        let actual2 = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                hide "from tar.xz" ;
                hide "from xz" ;
                hide "from tar" ;

                def "from tar" [] { 'opened tar' } ;
                def "from xz" [] { 'opened xz' } ;
                open file.tar.xz
            "#
        ));

        assert_eq!(actual2.out, "opened xz");
    })
}

#[test]
fn parses_dotfile() {
    Playground::setup("open_test_dotfile", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContent(
            ".gitignore",
            r#"
              /target/
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                hide "from gitignore" ;

                def "from gitignore" [] { 'opened gitignore' } ;
                open .gitignore
            "#
        ));

        assert_eq!(actual.out, "opened gitignore");
    })
}

#[test]
fn parses_csv() {
    Playground::setup("open_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu.zion.csv",
            r#"
                    author,lang,source
                    JT Turner,Rust,New Zealand
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

#[cfg(feature = "sqlite")]
#[test]
fn parses_sqlite() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            open sample.db
            | columns
            | length
        "
    ));

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn parses_sqlite_get_column_name() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            open sample.db
            | get strings
            | get x.0
        "
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
        "
            open caco3_plastics.tsv
            | first
            | get origin
        "
    ));

    assert_eq!(actual.out, "SPAIN")
}

#[test]
fn parses_json() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            open sgml_description.json
            | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
        "
    ));

    assert_eq!(actual.out, "markup")
}

#[test]
fn parses_xml() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        pipeline("
            open jt.xml
            | get content
            | where tag == channel
            | get content
            | flatten
            | where tag == item
            | get content
            | flatten
            | where tag == guid
            | get content.0.content.0
        ")
    );

    assert_eq!(actual.out, "https://www.jntrnr.com/off-to-new-adventures/")
}

#[cfg(feature = "dataframe")]
#[test]
fn parses_arrow_ipc() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            dfr open caco3_plastics.arrow
            | dfr into-nu
            | first
            | get origin
        "
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
    let expected = "File not found";

    assert!(
        actual.err.contains(expected),
        "Error:\n{}\ndoes not contain{}",
        actual.err,
        expected
    );
}

#[test]
fn open_wildcard() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
            open *.nu | where $it =~ echo | length
        "
    ));

    assert_eq!(actual.out, "3")
}

#[test]
fn open_multiple_files() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        "
        open caco3_plastics.csv caco3_plastics.tsv | get tariff_item | math sum
        "
    ));

    assert_eq!(actual.out, "58309279992")
}

#[test]
fn test_open_block_command() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
            def "from blockcommandparser" [] { lines | split column ",|," }
            let values = (open sample.blockcommandparser)
            print ($values | get column1 | get 0)
            print ($values | get column2 | get 0)
            print ($values | get column1 | get 1)
            print ($values | get column2 | get 1)
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
            "
                ls | find nu.zion | get 0 | get name | open $in
            "
        ));

        assert!(actual.err.is_empty());
    })
}

#[test]
fn open_no_parameter() {
    let actual = nu!("open");

    assert!(actual.err.contains("needs filename"));
}
