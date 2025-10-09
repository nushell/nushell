use std::path::PathBuf;

use nu_test_support::fs::Stub::EmptyFile;
use nu_test_support::fs::Stub::FileWithContent;
use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use rstest::rstest;

#[test]
fn parses_file_with_uppercase_extension() {
    Playground::setup("open_test_uppercase_extension", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
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

        let actual = nu!(cwd: dirs.test(), r#"
            open nu.zion.JSON
            | get glossary.GlossDiv.GlossList.GlossEntry.ID
        "#);

        assert_eq!(actual.out, "SGML");
    })
}

#[test]
fn parses_file_with_multiple_extensions() {
    Playground::setup("open_test_multiple_extensions", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("file.tar.gz", "this is a tar.gz file"),
            FileWithContent("file.tar.xz", "this is a tar.xz file"),
        ]);

        let actual = nu!(cwd: dirs.test(), r#"
            hide "from tar.gz" ;
            hide "from gz" ;
        
            def "from tar.gz" [] { 'opened tar.gz' } ;
            def "from gz" [] { 'opened gz' } ;
            open file.tar.gz
        "#);

        assert_eq!(actual.out, "opened tar.gz");

        let actual2 = nu!(cwd: dirs.test(), r#"
            hide "from tar.xz" ;
            hide "from xz" ;
            hide "from tar" ;
        
            def "from tar" [] { 'opened tar' } ;
            def "from xz" [] { 'opened xz' } ;
            open file.tar.xz
        "#);

        assert_eq!(actual2.out, "opened xz");
    })
}

#[test]
fn parses_dotfile() {
    Playground::setup("open_test_dotfile", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            ".gitignore",
            r#"
              /target/
            "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            hide "from gitignore" ;
        
            def "from gitignore" [] { 'opened gitignore' } ;
            open .gitignore
        "#);

        assert_eq!(actual.out, "opened gitignore");
    })
}

#[test]
fn parses_csv() {
    Playground::setup("open_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "nu.zion.csv",
            r#"
                    author,lang,source
                    JT Turner,Rust,New Zealand
                    Andres N. Robalino,Rust,Ecuador
                    Yehuda Katz,Rust,Estados Unidos
                "#,
        )]);

        let actual = nu!(cwd: dirs.test(), r#"
            open nu.zion.csv
            | where author == "Andres N. Robalino"
            | get source.0
        "#);

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
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | columns
        | length
    ");

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn parses_sqlite_get_column_name() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | get x.0
    ");

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
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open caco3_plastics.tsv
        | first
        | get origin
    ");

    assert_eq!(actual.out, "SPAIN")
}

#[test]
fn parses_json() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sgml_description.json
        | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
    ");

    assert_eq!(actual.out, "markup")
}

#[test]
fn parses_xml() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
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
    ");

    assert_eq!(actual.out, "https://www.jntrnr.com/off-to-new-adventures/")
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

    assert!(actual.err.contains("nu::shell::io::file_not_found"));
    assert!(
        actual.err.contains(
            &PathBuf::from_iter(["tests", "fixtures", "formats", "i_dont_exist.txt"])
                .display()
                .to_string()
        )
    );
}

#[test]
fn open_wildcard() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open *.nu | where $it =~ echo | length
    ");

    assert_eq!(actual.out, "3")
}

#[test]
fn open_multiple_files() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
    open caco3_plastics.csv caco3_plastics.tsv | get tariff_item | math sum
    ");

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
fn test_open_with_converter_flags() {
    // https://github.com/nushell/nushell/issues/13722
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        r#"
            def "from blockcommandparser" [ --flag ] { if $flag { "yes" } else { "no" } }
            open sample.blockcommandparser
        "#
    );

    assert_eq!(actual.out, "no")
}

#[test]
fn open_ignore_ansi() {
    Playground::setup("open_test_ansi", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("nu.zion.txt")]);

        let actual = nu!(cwd: dirs.test(), "
            ls | find nu.zion | get 0 | get name | open $in
        ");

        assert!(actual.err.is_empty());
    })
}

#[test]
fn open_no_parameter() {
    let actual = nu!("open");

    assert!(actual.err.contains("needs filename"));
}

#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn open_files_with_glob_metachars(#[case] src_name: &str) {
    Playground::setup("open_test_with_glob_metachars", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(src_name, "hello")]);

        let src = dirs.test().join(src_name);

        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "open '{}'",
                src.display(),
            )
        );

        assert!(actual.err.is_empty());
        assert!(actual.out.contains("hello"));

        // also test for variables.
        let actual = nu!(
            cwd: dirs.test(),
            format!(
                "let f = '{}'; open $f",
                src.display(),
            )
        );
        assert!(actual.err.is_empty());
        assert!(actual.out.contains("hello"));
    });
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn open_files_with_glob_metachars_nw(#[case] src_name: &str) {
    open_files_with_glob_metachars(src_name);
}

#[test]
fn open_files_inside_glob_metachars_dir() {
    Playground::setup("open_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent("test_file.txt", "hello")]);

        let actual = nu!(
            cwd: dirs.test().join(sub_dir),
            "open test_file.txt",
        );

        assert!(actual.err.is_empty());
        assert!(actual.out.contains("hello"));
    });
}

#[test]
fn test_content_types_with_open_raw() {
    Playground::setup("open_files_content_type_test", |dirs, _| {
        let result = nu!(cwd: dirs.formats(), "open --raw random_numbers.csv | metadata");
        assert!(result.out.contains("text/csv"));
        let result = nu!(cwd: dirs.formats(), "open --raw caco3_plastics.tsv | metadata");
        assert!(result.out.contains("text/tab-separated-values"));
        let result = nu!(cwd: dirs.formats(), "open --raw sample-simple.json | metadata");
        assert!(result.out.contains("application/json"));
        let result = nu!(cwd: dirs.formats(), "open --raw sample.ini | metadata");
        assert!(result.out.contains("text/plain"));
        let result = nu!(cwd: dirs.formats(), "open --raw sample_data.xlsx | metadata");
        assert!(result.out.contains("vnd.openxmlformats-officedocument"));
        let result = nu!(cwd: dirs.formats(), "open --raw sample_def.nu | metadata");
        assert!(result.out.contains("application/x-nuscript"));
        let result = nu!(cwd: dirs.formats(), "open --raw sample.eml | metadata");
        assert!(result.out.contains("message/rfc822"));
        let result = nu!(cwd: dirs.formats(), "open --raw cargo_sample.toml | metadata");
        assert!(result.out.contains("text/x-toml"));
        let result = nu!(cwd: dirs.formats(), "open --raw appveyor.yml | metadata");
        assert!(result.out.contains("application/yaml"));
    })
}

#[test]
fn test_metadata_without_raw() {
    Playground::setup("open_files_content_type_test", |dirs, _| {
        let result = nu!(cwd: dirs.formats(), "(open random_numbers.csv | metadata | get content_type?) == null");
        assert_eq!(result.out, "true");
        let result = nu!(cwd: dirs.formats(), "open random_numbers.csv | metadata | get source?");
        assert!(result.out.contains("random_numbers.csv"));
        let result = nu!(cwd: dirs.formats(), "(open caco3_plastics.tsv | metadata | get content_type?) == null");
        assert_eq!(result.out, "true");
        let result = nu!(cwd: dirs.formats(), "open caco3_plastics.tsv | metadata | get source?");
        assert!(result.out.contains("caco3_plastics.tsv"));
        let result = nu!(cwd: dirs.formats(), "(open sample-simple.json | metadata | get content_type?) == null");
        assert_eq!(result.out, "true");
        let result = nu!(cwd: dirs.formats(), "open sample-simple.json | metadata | get source?");
        assert!(result.out.contains("sample-simple.json"));
        // Only when not using nu_plugin_formats
        let result = nu!(cwd: dirs.formats(), "open sample.ini | metadata");
        assert!(result.out.contains("text/plain"));
        let result = nu!(cwd: dirs.formats(), "open sample.ini | metadata | get source?");
        assert!(result.out.contains("sample.ini"));
        let result = nu!(cwd: dirs.formats(), "(open sample_data.xlsx | metadata | get content_type?) == null");
        assert_eq!(result.out, "true");
        let result = nu!(cwd: dirs.formats(), "open sample_data.xlsx | metadata | get source?");
        assert!(result.out.contains("sample_data.xlsx"));
        let result = nu!(cwd: dirs.formats(), "open sample_def.nu | metadata | get content_type?");
        assert_eq!(result.out, "application/x-nuscript");
        let result = nu!(cwd: dirs.formats(), "open sample_def.nu | metadata | get source?");
        assert!(result.out.contains("sample_def"));
        // Only when not using nu_plugin_formats
        let result = nu!(cwd: dirs.formats(), "open sample.eml | metadata | get content_type?");
        assert_eq!(result.out, "message/rfc822");
        let result = nu!(cwd: dirs.formats(), "open sample.eml | metadata | get source?");
        assert!(result.out.contains("sample.eml"));
        let result = nu!(cwd: dirs.formats(), "(open cargo_sample.toml | metadata | get content_type?) == null");
        assert_eq!(result.out, "true");
        let result = nu!(cwd: dirs.formats(), "open cargo_sample.toml | metadata | get source?");
        assert!(result.out.contains("cargo_sample.toml"));
        let result =
            nu!(cwd: dirs.formats(), "(open appveyor.yml | metadata | get content_type?) == null");
        assert_eq!(result.out, "true");
        let result = nu!(cwd: dirs.formats(), "open appveyor.yml | metadata | get source?");
        assert!(result.out.contains("appveyor.yml"));
    })
}
