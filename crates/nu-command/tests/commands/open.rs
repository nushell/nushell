use std::path::Path;

use nu_protocol::shell_error;
use nu_test_support::{
    fs::Stub::{EmptyFile, FileWithContent, FileWithContentToBeTrimmed},
    playground::Playground,
    prelude::*,
};
use rstest::rstest;

#[test]
fn parses_file_with_uppercase_extension() -> Result {
    Playground::setup("open_test_uppercase_extension", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "nu.zion.JSON",
            r#"
                {
                    "glossary": {
                        "GlossDiv": {
                            "GlossList": {
                                "GlossEntry": {
                                    "ID": "SGML"
                                }
                            }
                        }
                    }
                }
            "#,
        )]);

        let code = "
            open nu.zion.JSON
            | get glossary.GlossDiv.GlossList.GlossEntry.ID
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq("SGML")
    })
}

#[test]
fn parses_file_with_tar_gz_extension() -> Result {
    Playground::setup("open_test_tar_gz_extension", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("file.tar.gz", "this is a tar.gz file")]);

        let code = r#"
            hide "from tar.gz" ;
            hide "from gz" ;

            def "from tar.gz" [] { 'opened tar.gz' } ;
            def "from gz" [] { 'opened gz' } ;
            open file.tar.gz
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("opened tar.gz")
    })
}

#[test]
fn parses_file_with_tar_xz_extension() -> Result {
    Playground::setup("open_test_tar_xz_extension", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("file.tar.xz", "this is a tar.xz file")]);

        let code = r#"
            hide "from tar.xz" ;
            hide "from xz" ;
            hide "from tar" ;

            def "from tar" [] { 'opened tar' } ;
            def "from xz" [] { 'opened xz' } ;
            open file.tar.xz
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("opened xz")
    })
}

#[test]
fn parses_dotfile() -> Result {
    Playground::setup("open_test_dotfile", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(".gitignore", "/target/")]);

        let code = r#"
            hide "from gitignore" ;

            def "from gitignore" [] { 'opened gitignore' } ;
            open .gitignore
        "#;

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("opened gitignore")
    })
}

#[test]
fn parses_csv() -> Result {
    Playground::setup("open_test_1", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "nu.zion.csv",
            "
                author,lang,source
                JT Turner,Rust,New Zealand
                Andres N. Robalino,Rust,Ecuador
                Yehuda Katz,Rust,Estados Unidos
            ",
        )]);

        let code = r#"
            open nu.zion.csv
            | where author == "Andres N. Robalino"
            | get source.0
        "#;

        test().cwd(dirs.test()).run(code).expect_value_eq("Ecuador")
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
#[rstest]
#[case::columns("columns | length", 3)]
fn sqlite_database_operations(#[case] operation: &str, #[case] expected: impl IntoValue) -> Result {
    let mut tester = test().cwd("tests/fixtures/formats");
    let sample: Value = tester.run("open sample.db")?;
    tester
        .run_with_data(operation, sample)
        .expect_value_eq(expected)
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case::columns("columns | first", "z")]
#[case::values("values | first | first", 1)]
#[case::generic_filter("take 2 | update z {|row| $row.z + 1 } | get z.0", 2)]
#[case::sort("sort | first | get z", 1)]
#[case::headers("headers | columns | first", "1")]
#[case::drop_column("drop column | first | columns | length", 0)]
#[case::roll_up("roll up | first | get z", 42)]
#[case::roll_down("roll down | first | get z", ())]
#[case::chunks("chunks 2 | length", 3)]
#[case::window("window 2 | length", 4)]
#[case::reverse("reverse | last | get z", 1)]
#[case::reject("reject z | first | columns | length", 0)]
#[case::drop_nth("drop nth 1 | get z.1", 425)]
#[case::compact("compact z | length", 4)]
#[case::rename("rename n | columns | first", "n")]
#[case::transpose("transpose | columns | first", "column0")]
#[case::zip("zip [1 2 3 4 5] | length", 5)]
#[case::enumerate("enumerate | first | get index", 0)]
#[case::flatten("flatten | first | get z", 1)]
#[case::append("append {z: 99} | last | get z", 99)]
#[case::prepend("prepend {z: 0} | first | get z", 0)]
#[case::reduce("reduce -f 0 {|row, acc| $acc + ($row.z | default 0) }", 4721)]
#[case::each("each {|row| $row.z } | length", 4)]
#[case::par_each("par-each {|row| $row.z } | length", 4)]
#[case::upsert("upsert z 0 | last | get z", 0)]
#[case::default("default 0 z | last | get z", 0)]
#[case::update_cells("update cells {default (-1) | $in + 1 } | last | get z", 0)]
#[case::every("every 2 | length", 3)]
#[case::first("first 2 | get z.1", 42)]
#[case::last("last 2 | get z.0", 4253)]
#[case::take("take 3 | length", 3)]
#[case::skip("skip 2 | get z.0", 425)]
#[case::slice("slice 1..3 | get z.1", 425)]
#[case::select("select z | columns | first", "z")]
#[case::where_("where z > 100 | length", 2)]
#[case::sort_by("sort-by z | get z.1", 42)]
#[case::uniq("uniq | length", 5)]
#[case::uniq_by("uniq-by z | length", 5)]
#[case::is_empty("is-empty", false)]
#[case::is_not_empty("is-not-empty", true)]
#[case::all("all {|row| $row.z != null }", false)]
#[case::any("any {|row| $row.z == null }", true)]
#[case::take_while("take while {|row| $row.z < 1000 } | length", 3)]
#[case::skip_while("skip while {|row| $row.z < 1000 } | first | get z", 4253)]
#[case::take_until("take until {|row| $row.z > 1000 } | length", 3)]
#[case::skip_until("skip until {|row| $row.z > 1000 } | first | get z", 4253)]
#[case::insert("insert n 1 | first | get n", 1)]
#[case::update("update z 0 | first | get z", 0)]
#[case::wrap("wrap wrapped | columns | first", "wrapped")]
#[case::interleave("interleave { [{z: 999}] } | length", 6)]
#[case::rotate("rotate --ccw | columns | first", "column0")]
#[case::group_by("group-by z | length", 5)]
#[case::get("get z.0", 1)]
#[case::length("length", 5)]
#[case::merge("first | merge {n: 1} | get n", 1)]
#[case::merge_deep("first | merge deep {n: {x: 1}} | get n.x", 1)]
#[case::items("first | items {|k, v| $k } | length", 1)]
#[case::chunk_by("chunk-by {|row| (($row.z | default 0) mod 2) } | length", 4)]
#[case::drop("drop 1 | length", 4)]
#[case::shuffle("shuffle | length", 5)]
#[case::split_list("split list 2 | flatten | length", 5)]
#[case::each_while("each while {|row| $row } | length", 5)]
#[case::tee("tee {|x| $x | ignore } | flatten | length", 6)]
#[case::filter("filter {|row| $row.z > 100 } | length", 2)]
fn sqlite_int_table_operations(
    #[case] operation: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let mut tester = test().cwd("tests/fixtures/formats");
    let sample: Value = tester.run("open sample.db | get ints")?;
    tester
        .run_with_data(operation, sample)
        .expect_value_eq(expected)
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case::get_column("get x.0", "hello")]
#[case::move_("move x --after y | columns | first", "y")]
#[case::find("find --no-highlight hello | get x.1", "hello")]
#[case::roll_left("roll left | columns | first", "y")]
#[case::roll_right("roll right | columns | first", "y")]
#[case::join("join [[x tag]; [hello a] [nushell b]] x | length", 4)]
fn sqlite_string_table_operations(
    #[case] operation: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    let mut tester = test().cwd("tests/fixtures/formats");
    let sample: Value = tester.run("open sample.db | get strings")?;
    tester
        .run_with_data(operation, sample)
        .expect_value_eq(expected)
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_lines_errors_with_type_mismatch() -> Result {
    let mut tester = test().cwd("tests/fixtures/formats");
    let sample: Value = tester.run("open sample.db | get ints")?;
    tester
        .run_with_data("lines", sample)
        .expect_error_code_eq("nu::shell::only_supports_this_input_type")
}

#[test]
fn parses_toml() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open cargo_sample.toml | get package.edition")
        .expect_value_eq("2018")
}

#[test]
fn parses_tsv() -> Result {
    let code = "
        open caco3_plastics.tsv
        | first
        | get origin
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("SPAIN")
}

#[test]
fn parses_json() -> Result {
    let code = "
        open sgml_description.json
        | get glossary.GlossDiv.GlossList.GlossEntry.GlossSee
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("markup")
}

#[test]
fn parses_xml() -> Result {
    let code = "
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
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("https://www.jntrnr.com/off-to-new-adventures/")
}

#[test]
fn error_if_file_not_found() -> Result {
    let dir = Path::new("tests/fixtures/formats");

    let err = test()
        .cwd(dir)
        .run("open i_dont_exist.txt")
        .expect_io_error()?;

    assert_eq!(err.kind, shell_error::io::ErrorKind::FileNotFound);
    assert!(
        err.path
            .expect("error should include path")
            .ends_with(dir.join("i_dont_exist.txt"))
    );
    Ok(())
}

#[test]
fn open_wildcard() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open *.nu | where $it =~ echo | length")
        .expect_value_eq(3)
}

#[test]
fn open_multiple_files() -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run("open caco3_plastics.csv caco3_plastics.tsv | get tariff_item | math sum")
        .expect_value_eq(58309279992i64)
}

#[test]
fn test_open_block_command() -> Result {
    let code = r#"
        def "from blockcommandparser" [] { lines | split column ",|," }
        open sample.blockcommandparser
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(test_table![
            ["column0", "column1"];
            ["a", "b"],
            ["c", "d"],
        ])
}

#[test]
fn test_open_with_converter_flags() -> Result {
    // https://github.com/nushell/nushell/issues/13722
    let code = r#"
        def "from blockcommandparser" [ --flag ] { if $flag { "yes" } else { "no" } }
        open sample.blockcommandparser
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("no")
}

#[test]
fn open_ignore_ansi() -> Result {
    Playground::setup("open_test_ansi", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("nu.zion.txt")]);
        let code = "ls | find nu.zion | get 0 | get name | open $in";
        let _: Value = test().cwd(dirs.test()).run(code)?;
        Ok(())
    })
}

#[test]
fn open_no_parameter() -> Result {
    let err = test().run("open").expect_shell_error()?;
    assert_contains("needs filename", err.to_string());
    Ok(())
}

#[track_caller]
#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn open_literal_file_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("open_test_with_glob_metachars", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(src_name, "hello")]);
        let src = dirs.test().join(src_name);
        test()
            .cwd(dirs.test())
            .run(format!("open '{}'", src.display()))
            .expect_value_eq("hello")
    })
}

#[track_caller]
#[rstest]
#[case("a]c")]
#[case("a[c")]
#[case("a[bc]d")]
#[case("a][c")]
fn open_variable_file_with_glob_metachars(#[case] src_name: &str) -> Result {
    Playground::setup("open_test_variable_with_glob_metachars", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(src_name, "hello")]);
        let src = dirs.test().join(src_name);
        test()
            .cwd(dirs.test())
            .run(format!("let f = '{}'; open $f", src.display()))
            .expect_value_eq("hello")
    })
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn open_literal_files_with_glob_metachars_nw(#[case] src_name: &str) -> Result {
    open_literal_file_with_glob_metachars(src_name)
}

#[cfg(not(windows))]
#[rstest]
#[case("a]?c")]
#[case("a*.?c")]
// windows doesn't allow filename with `*`.
fn open_variable_files_with_glob_metachars_nw(#[case] src_name: &str) -> Result {
    open_variable_file_with_glob_metachars(src_name)
}

#[test]
fn open_files_inside_glob_metachars_dir() -> Result {
    Playground::setup("open_files_inside_glob_metachars_dir", |dirs, sandbox| {
        let sub_dir = "test[]";
        sandbox
            .within(sub_dir)
            .with_files(&[FileWithContent("test_file.txt", "hello")]);

        test()
            .cwd(dirs.test().join(sub_dir))
            .run("open test_file.txt")
            .expect_value_eq("hello")
    })
}

#[rstest]
#[case::csv("random_numbers.csv", "text/csv")]
#[case::tsv("caco3_plastics.tsv", "text/tab-separated-values")]
#[case::json("sample-simple.json", "application/json")]
#[case::ini("sample.ini", "text/plain")]
#[case::xlsx("sample_data.xlsx", "vnd.openxmlformats-officedocument")]
#[case::nu("sample_def.nu", "application/x-nuscript")]
#[case::eml("sample.eml", "message/rfc822")]
#[case::toml("cargo_sample.toml", "text/x-toml")]
#[case::yaml("appveyor.yml", "application/yaml")]
fn test_content_types_with_open_raw(#[case] file: &str, #[case] content_type: &str) -> Result {
    let actual: String = test()
        .cwd("tests/fixtures/formats")
        .run(format!("open --raw {file} | metadata | get content_type"))?;

    assert_contains(content_type, actual);
    Ok(())
}

#[rstest]
#[case::csv("random_numbers.csv")]
#[case::tsv("caco3_plastics.tsv")]
#[case::json("sample-simple.json")]
#[case::xlsx("sample_data.xlsx")]
#[case::toml("cargo_sample.toml")]
#[case::yaml("appveyor.yml")]
fn test_metadata_without_raw_has_no_content_type(#[case] file: &str) -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run(format!(
            "(open {file} | metadata | get content_type?) == null"
        ))
        .expect_value_eq(true)
}

#[rstest]
#[case::csv("random_numbers.csv")]
#[case::tsv("caco3_plastics.tsv")]
#[case::json("sample-simple.json")]
#[case::xlsx("sample_data.xlsx")]
#[case::toml("cargo_sample.toml")]
#[case::yaml("appveyor.yml")]
fn test_metadata_without_raw_has_source(#[case] file: &str) -> Result {
    let source: String = test()
        .cwd("tests/fixtures/formats")
        .run(format!("open {file} | metadata | get source?"))?;
    assert_contains(file, source);
    Ok(())
}

#[rstest]
#[case::nu("sample_def.nu", "application/x-nuscript")]
// Only when not using nu_plugin_formats
#[case::ini("sample.ini", "text/plain")]
#[case::eml("sample.eml", "message/rfc822")]
fn test_metadata_without_raw_has_content_type(
    #[case] file: &str,
    #[case] content_type: &str,
) -> Result {
    test()
        .cwd("tests/fixtures/formats")
        .run(format!("open {file} | metadata | get content_type?"))
        .expect_value_eq(content_type)
}

#[rstest]
#[case::nu("sample_def.nu", "sample_def")]
// Only when not using nu_plugin_formats
#[case::ini("sample.ini", "sample.ini")]
#[case::eml("sample.eml", "sample.eml")]
fn test_metadata_without_raw_has_source_for_files_with_content_type(
    #[case] file: &str,
    #[case] source_contains: &str,
) -> Result {
    let source: String = test()
        .cwd("tests/fixtures/formats")
        .run(format!("open {file} | metadata | get source?"))?;
    assert_contains(source_contains, source);
    Ok(())
}
