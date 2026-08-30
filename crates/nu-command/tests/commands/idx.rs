use nu_test_support::prelude::*;

#[test]
#[serial]
fn idx_init_sets_initialized_status(playground: Playground) -> Result {
    playground.empty_file("alpha.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init . | get initialized")
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_status_reports_initialized_after_init(playground: Playground) -> Result {
    playground.empty_file("beta.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init .; idx status | get initialized")
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_status_reports_watch_enabled_by_default(playground: Playground) -> Result {
    playground.empty_file("beta.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init .; idx status | get watch")
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_status_reports_scan_duration_as_duration(playground: Playground) -> Result {
    playground.empty_file("timed.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx status | get scan_duration | describe")
        .expect_value_eq("duration")
}

#[test]
#[serial]
fn idx_files_returns_records_with_full_path(playground: Playground) -> Result {
    playground.empty_file("gamma.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx files | get 0.full_path | str contains gamma.txt")
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_files_returns_ext_and_native_types(playground: Playground) -> Result {
    playground.empty_file("quote.txt")?;

    test()
            .cwd(playground.path())
            .run("idx init . --wait; let row = (idx files quote | where file_name == quote.txt | first); let modified_kind = ($row.modified | describe | str downcase); ($row.ext == 'txt') and (($row.size | describe) == 'filesize') and ($modified_kind | str contains 'date')")
            .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_init_wait_reports_scanning_as_false(playground: Playground) -> Result {
    playground.empty_file("scan-me.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait | get scanning")
        .expect_value_eq(false)
}

#[test]
#[serial]
fn idx_init_wait_indexes_generated_files_before_returning(playground: Playground) -> Result {
    test()
            .cwd(playground.path())
            .run("let expected = 600; 0..($expected - 1) | each {|i| touch $\"bulk-($i).txt\" }; idx init . --wait; idx files | where {|row| $row.file_name | str starts-with 'bulk-' } | length")
            .expect_value_eq(600)
}

#[test]
#[serial]
fn idx_init_wait_status_reports_indexed_file_count(playground: Playground) -> Result {
    playground.empty_file("alpha.txt")?;
    playground.empty_file("beta.txt")?;
    playground.empty_file("gamma.txt")?;

    test()
            .cwd(playground.path())
            .run("let status = (idx init . --wait); let counted = (idx files | length); ($status | get files) == $counted")
            .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_files_optional_query_uses_fuzzy_matching(playground: Playground) -> Result {
    playground.dir("src")?;
    playground.empty_file("src/main.rs")?;
    playground.empty_file("src/lib.rs")?;
    playground.empty_file("README.md")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx files mai | where file_name == main.rs | length")
        .expect_value_eq(1)
}

#[test]
#[serial]
fn idx_dirs_returns_records_with_full_path(playground: Playground) -> Result {
    playground.dir("nested")?;
    playground.empty_file("nested/delta.txt")?;

    test()
            .cwd(playground.path())
            .run("idx init . --wait; idx dirs | get full_path | any {|path| $path | str contains 'nested' }")
            .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_dirs_optional_query_filters_results(playground: Playground) -> Result {
    playground.dir("src/components")?;
    playground.dir("tests/fixtures")?;
    playground.empty_file("src/components/widget.nu")?;
    playground.empty_file("tests/fixtures/spec.nu")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx dirs comp | get relative_path | any {|path| ($path | str contains 'src/components') or ($path | str contains 'src\\components') }")
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_find_defaults_to_files_and_dirs(playground: Playground) -> Result {
    playground.dir("target-dir")?;
    playground.empty_file("target-file.txt")?;
    playground.empty_file("target-dir/inside.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; let rows = (idx find target); [($rows | where kind == file | length) ($rows | where kind == dir | length)] | to nuon")
        .expect_value_eq("[2, 1]")
}

#[test]
#[serial]
fn idx_limits_must_be_non_negative() -> Result {
    for command in ["idx find target --limit -1", "idx search target --limit -1"] {
        let err = test().run(command).expect_error()?;
        assert!(matches!(err, ShellError::NeedsPositiveValue { .. }));
    }
    Ok(())
}

#[test]
#[serial]
fn idx_live_views_exclude_tombstones(playground: Playground) -> Result {
    playground.dir("removed-tree")?;
    playground.empty_file("kept.txt")?;
    playground.empty_file("removed-tree/deleted.txt")?;

    let mut tester = test().cwd(playground.path());

    // Wait through fuzzy queries (the watcher applies removals asynchronously)
    let (): () = tester.run(
            r#"
                idx init . --wait --no-content-indexing | ignore
                rm --recursive removed-tree
                mut attempts = 0
                loop {
                    if (idx find deleted --files | is-empty) and (idx find removed-tree --dirs | is-empty) { break }
                    if $attempts >= 100 { error make { msg: "idx watcher did not process deletion" } }
                    $attempts += 1
                    sleep 20ms
                }
            "#,
        )?;

    // Unfiltered views/status should only show live entries.
    tester
        .run(
            r#"
                let files = idx files
                let dirs = idx dirs
                let status = idx status
                [
                    (($files | where file_name == "deleted.txt") | is-empty)
                    (($dirs | where relative_path =~ "removed-tree") | is-empty)
                    ($status.files == ($files | length))
                    ($status.dirs == ($dirs | length))
                ] | all {|value| $value }
            "#,
        )
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_watched_runtime_uses_one_live_picker(playground: Playground) -> Result {
    playground.empty_file("seed.txt")?;

    let mut tester = test().cwd(playground.path());

    let (): () = tester.run("idx init . --wait --no-content-indexing | ignore")?;

    // Once content search sees the watcher update, every live view should see the same file and dir.
    tester
            .run(
                r#"
                    let before = idx status
                    mkdir watched-dir
                    "watchprobe" | save watched-dir/watched.txt
                    mut attempts = 0
                    loop {
                        if not (idx search watchprobe | is-empty) { break }
                        if $attempts >= 100 { error make { msg: "idx watcher did not process creation" } }
                        $attempts += 1
                        sleep 20ms
                    }
                    
                    let status = idx status
                    [
                        (idx files watched | length)
                        (idx dirs watched | length)
                        (idx find watched --files | length)
                        (idx find watched --dirs | length)
                        ($status.files - $before.files)
                        ($status.dirs - $before.dirs)
                    ] | all {|count| $count > 0 }
                "#,
            )
            .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_search_finds_content(playground: Playground) -> Result {
    playground.file("searchable.txt", "hello from idx search")?;
    playground.file("other.txt", "unrelated")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx search hello | get 0.relative_path | str contains searchable.txt")
        .expect_value_eq(true)
}

#[test]
#[serial]
fn idx_search_uses_relative_path_from_current_directory(playground: Playground) -> Result {
    playground.dir("src")?;
    playground.file("src/main.rs", "pattern found here")?;

    test()
        .cwd(playground.path())
        .run("cd src; idx init .. --wait; idx search pattern | get 0.relative_path")
        .expect_value_eq("main.rs")
}

#[test]
#[serial]
fn idx_find_uses_relative_path_from_current_directory(playground: Playground) -> Result {
    playground.dir("src")?;
    playground.empty_file("src/main.rs")?;

    test()
        .cwd(playground.path())
        .run("cd src; idx init .. --wait; idx find main | where kind == file | get 0.relative_path")
        .expect_value_eq("main.rs")
}

#[test]
#[serial]
fn idx_search_bracket_pattern_finds_content(playground: Playground) -> Result {
    playground.file("test.txt", "Lyrics[")?;
    playground.file("other.txt", "unrelated")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx search 'Lyrics[' | length")
        .expect_value_eq(1)
}

#[test]
#[serial]
fn idx_search_question_mark_as_literal_text(playground: Playground) -> Result {
    playground.file("code.rs", "foo? bar")?;
    playground.file("other.rs", "just foo")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx search 'foo?' | length")
        .expect_value_eq(1)
}

#[test]
#[serial]
fn idx_search_bracket_literal_example_finds_content(playground: Playground) -> Result {
    playground.file("example.rs", "arr[0] = value")?;
    playground.file("other.txt", "unrelated")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx search 'arr[0]' | length")
        .expect_value_eq(1)
}

#[test]
#[serial]
fn idx_search_glob_with_path_separator_example_filters_files(playground: Playground) -> Result {
    playground.file("tests/search_test.rs", "pattern found here")?;
    playground.file("src/main.rs", "pattern found here")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx search pattern tests/* | length")
        .expect_value_eq(1)
}

#[test]
#[serial]
fn idx_search_brace_glob_still_filters_files(playground: Playground) -> Result {
    playground.file("alpha.rs", "pattern found here")?;
    playground.file("beta.js", "pattern found here")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx search pattern *.{rs,js} | length")
        .expect_value_eq(2)
}

#[test]
#[serial]
fn idx_drop_clears_runtime(playground: Playground) -> Result {
    playground.empty_file("alpha.txt")?;

    test()
        .cwd(playground.path())
        .run("idx init . --wait; idx drop | get dropped")
        .expect_value_eq(true)?;

    test()
        .cwd(playground.path())
        .run("idx status | get initialized")
        .expect_value_eq(false)
}
