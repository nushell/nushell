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

        let actual = nu!(cwd: dirs.test(), "
            open nu.zion.JSON
            | get glossary.GlossDiv.GlossList.GlossEntry.ID
        ");

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
            "
              /target/
            ",
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
            "
                    author,lang,source
                    JT Turner,Rust,New Zealand
                    Andres N. Robalino,Rust,Ecuador
                    Yehuda Katz,Rust,Estados Unidos
                ",
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

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_columns_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | columns
        | first
    ");

    assert_eq!(actual.out, "z");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_values_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | values
        | first
        | first
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_generic_filters_work() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | take 2
        | update z {|row| $row.z + 1 }
        | get z.0
    ");

    assert_eq!(actual.out, "2");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_sort_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | sort
        | first
        | get z
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_headers_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | headers
        | columns
        | first
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_move_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | move x --after y
        | columns
        | first
    ");

    assert_eq!(actual.out, "y");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_drop_column_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | drop column
        | first
        | columns
        | length
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_roll_up_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | roll up
        | first
        | get z
    ");

    assert_eq!(actual.out, "42");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_roll_down_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | roll down
        | first
        | get z
    ");

    assert_eq!(actual.out, "");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_roll_left_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | roll left
        | columns
        | first
    ");

    assert_eq!(actual.out, "y");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_roll_right_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | roll right
        | columns
        | first
    ");

    assert_eq!(actual.out, "y");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_default_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | default 0 z
        | last
        | get z
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_chunks_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | chunks 2
        | length
    ");

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_window_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | window 2
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_reverse_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | reverse
        | last
        | get z
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_reject_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | reject z
        | first
        | columns
        | length
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_drop_nth_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | drop nth 1
        | get z.1
    ");

    assert_eq!(actual.out, "425");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_compact_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | compact z
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_rename_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | rename n
        | columns
        | first
    ");

    assert_eq!(actual.out, "n");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_find_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | find --no-highlight hello
        | get x.1
    ");

    assert_eq!(actual.out, "hello");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_transpose_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | transpose
        | columns
        | first
    ");

    assert_eq!(actual.out, "column0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_zip_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | zip [1 2 3 4 5]
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_enumerate_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | enumerate
        | first
        | get index
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_flatten_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | flatten
        | first
        | get z
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_append_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | append {z: 99}
        | last
        | get z
    ");

    assert_eq!(actual.out, "99");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_prepend_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | prepend {z: 0}
        | first
        | get z
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_reduce_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | reduce -f 0 {|row, acc| $acc + ($row.z | default 0) }
    ");

    assert_eq!(actual.out, "4721");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_each_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | each {|row| $row.z }
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_par_each_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | par-each {|row| $row.z }
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_upsert_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | upsert z 0
        | last
        | get z
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_update_cells_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | update cells {|v| if $v == null { 0 } else { $v + 1 } }
        | last
        | get z
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_every_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | every 2
        | length
    ");

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_first_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first 2
        | get z.1
    ");

    assert_eq!(actual.out, "42");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_last_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | last 2
        | get z.0
    ");

    assert_eq!(actual.out, "4253");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_take_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | take 3
        | length
    ");

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_skip_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | skip 2
        | get z.0
    ");

    assert_eq!(actual.out, "425");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_slice_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | slice 1..3
        | get z.1
    ");

    assert_eq!(actual.out, "425");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_select_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | select z
        | columns
        | first
    ");

    assert_eq!(actual.out, "z");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_where_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | where z > 100
        | length
    ");

    assert_eq!(actual.out, "2");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_sort_by_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | sort-by z
        | get z.1
    ");

    assert_eq!(actual.out, "42");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_uniq_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | uniq
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_uniq_by_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | uniq-by z
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_is_empty_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | is-empty
    ");

    assert_eq!(actual.out, "false");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_is_not_empty_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | is-not-empty
    ");

    assert_eq!(actual.out, "true");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_all_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | all {|row| $row.z != null }
    ");

    assert_eq!(actual.out, "false");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_any_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | any {|row| $row.z == null }
    ");

    assert_eq!(actual.out, "true");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_take_while_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | take while {|row| $row.z < 1000 }
        | length
    ");

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_skip_while_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | skip while {|row| $row.z < 1000 }
        | first
        | get z
    ");

    assert_eq!(actual.out, "4253");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_take_until_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | take until {|row| $row.z > 1000 }
        | length
    ");

    assert_eq!(actual.out, "3");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_skip_until_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | skip until {|row| $row.z > 1000 }
        | first
        | get z
    ");

    assert_eq!(actual.out, "4253");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_insert_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | insert n 1
        | first
        | get n
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_update_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | update z 0
        | first
        | get z
    ");

    assert_eq!(actual.out, "0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_wrap_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | wrap wrapped
        | columns
        | first
    ");

    assert_eq!(actual.out, "wrapped");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_interleave_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | interleave { [{z: 999}] }
        | length
    ");

    assert_eq!(actual.out, "6");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_rotate_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | rotate --ccw
        | columns
        | first
    ");

    assert_eq!(actual.out, "column0");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_group_by_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | group-by z
        | columns
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_get_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | get z.0
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_length_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_merge_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first
        | merge {n: 1}
        | get n
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_merge_deep_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first
        | merge deep {n: {x: 1}}
        | get n.x
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_items_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first
        | items {|k, v| $k }
        | length
    ");

    assert_eq!(actual.out, "1");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_chunk_by_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | chunk-by {|row| (($row.z | default 0) mod 2) }
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_join_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | join [[x tag]; [hello a] [nushell b]] x
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_drop_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | drop 1
        | length
    ");

    assert_eq!(actual.out, "4");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_shuffle_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | shuffle
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_split_list_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | split list 2
        | flatten
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_each_while_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | each while {|row| $row }
        | length
    ");

    assert_eq!(actual.out, "5");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_tee_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | tee {|x| $x | ignore }
        | flatten
        | length
    ");

    assert_eq!(actual.out, "6");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_filter_works() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | filter {|row| $row.z > 100 }
        | length
    ");

    assert_eq!(actual.out, "2");
}

#[cfg(feature = "sqlite")]
#[test]
fn sqlite_get_table_lines_errors_with_type_mismatch() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | lines
    ");

    assert!(
        actual
            .err
            .contains("nu::shell::only_supports_this_input_type")
    );
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
            print ($values | get column0 | get 0)
            print ($values | get column1 | get 0)
            print ($values | get column0 | get 1)
            print ($values | get column1 | get 1)
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
