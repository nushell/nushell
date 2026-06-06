use nu_test_support::fs::Stub::{EmptyFile, FileWithContent};
use nu_test_support::prelude::*;

#[test]
#[serial]
fn idx_init_sets_initialized_status() -> Result {
    Playground::setup("idx_init_sets_initialized_status", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("alpha.txt")]);

        test()
            .cwd(dirs.test())
            .run("idx init . | get initialized")
            .expect_value_eq(true)
    })
}

#[test]
#[serial]
fn idx_status_reports_initialized_after_init() -> Result {
    Playground::setup(
        "idx_status_reports_initialized_after_init",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("beta.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init .; idx status | get initialized")
                .expect_value_eq(true)
        },
    )
}

#[test]
#[serial]
fn idx_status_reports_watch_enabled_by_default() -> Result {
    Playground::setup(
        "idx_status_reports_watch_enabled_by_default",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("beta.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init .; idx status | get watch")
                .expect_value_eq(true)
        },
    )
}

#[test]
#[serial]
fn idx_status_reports_scan_duration_as_duration() -> Result {
    Playground::setup(
        "idx_status_reports_scan_duration_as_duration",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("timed.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait; idx status | get scan_duration | describe")
                .expect_value_eq("duration")
        },
    )
}

#[test]
#[serial]
fn idx_files_returns_records_with_full_path() -> Result {
    Playground::setup(
        "idx_files_returns_records_with_full_path",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("gamma.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait; idx files | get 0.full_path | str contains gamma.txt")
                .expect_value_eq(true)
        },
    )
}

#[test]
#[serial]
fn idx_files_returns_ext_and_native_types() -> Result {
    Playground::setup("idx_files_returns_ext_and_native_types", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("quote.txt")]);

        test()
                .cwd(dirs.test())
                .run("idx init . --wait; let row = (idx files quote | where file_name == quote.txt | first); let modified_kind = ($row.modified | describe | str downcase); ($row.ext == 'txt') and (($row.size | describe) == 'filesize') and ($modified_kind | str contains 'date')")
                .expect_value_eq(true)
    })
}

#[test]
#[serial]
fn idx_init_wait_reports_scanning_as_false() -> Result {
    Playground::setup(
        "idx_init_wait_reports_scanning_as_false",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("scan-me.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait | get scanning")
                .expect_value_eq(false)
        },
    )
}

#[test]
#[serial]
fn idx_init_wait_indexes_generated_files_before_returning() -> Result {
    Playground::setup(
        "idx_init_wait_indexes_generated_files_before_returning",
        |dirs, _sandbox| {
            test()
                .cwd(dirs.test())
                .run("let expected = 600; 0..($expected - 1) | each {|i| touch $\"bulk-($i).txt\" }; idx init . --wait; idx files | where {|row| $row.file_name | str starts-with 'bulk-' } | length")
                .expect_value_eq(600)
        },
    )
}

#[test]
#[serial]
fn idx_init_wait_status_reports_indexed_file_count() -> Result {
    Playground::setup(
        "idx_init_wait_status_reports_indexed_file_count",
        |dirs, sandbox| {
            sandbox.with_files(&[
                EmptyFile("alpha.txt"),
                EmptyFile("beta.txt"),
                EmptyFile("gamma.txt"),
            ]);

            test()
                .cwd(dirs.test())
                .run("let status = (idx init . --wait); let counted = (idx files | length); ($status | get files) == $counted")
                .expect_value_eq(true)
        },
    )
}

#[test]
#[serial]
fn idx_files_optional_query_uses_fuzzy_matching() -> Result {
    Playground::setup(
        "idx_files_optional_query_uses_fuzzy_matching",
        |dirs, sandbox| {
            sandbox.mkdir("src");
            sandbox.with_files(&[
                EmptyFile("src/main.rs"),
                EmptyFile("src/lib.rs"),
                EmptyFile("README.md"),
            ]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait; idx files mai | where file_name == main.rs | length")
                .expect_value_eq(1)
        },
    )
}

#[test]
#[serial]
fn idx_dirs_returns_records_with_full_path() -> Result {
    Playground::setup(
        "idx_dirs_returns_records_with_full_path",
        |dirs, sandbox| {
            sandbox.mkdir("nested");
            sandbox.with_files(&[EmptyFile("nested/delta.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait; idx dirs | get full_path | any {|path| $path | str contains 'nested' }")
                .expect_value_eq(true)
        },
    )
}

#[test]
#[serial]
fn idx_dirs_optional_query_filters_results() -> Result {
    Playground::setup(
        "idx_dirs_optional_query_filters_results",
        |dirs, sandbox| {
            sandbox.mkdir("src/components");
            sandbox.mkdir("tests/fixtures");
            sandbox.with_files(&[
                EmptyFile("src/components/widget.nu"),
                EmptyFile("tests/fixtures/spec.nu"),
            ]);

            test()
            .cwd(dirs.test())
            .run("idx init . --wait; idx dirs comp | get relative_path | any {|path| ($path | str contains 'src/components') or ($path | str contains 'src\\components') }")
            .expect_value_eq(true)
        },
    )
}

#[test]
#[serial]
fn idx_find_defaults_to_files_and_dirs() -> Result {
    Playground::setup("idx_find_defaults_to_files_and_dirs", |dirs, sandbox| {
        sandbox.mkdir("target-dir");
        sandbox.with_files(&[
            EmptyFile("target-file.txt"),
            EmptyFile("target-dir/inside.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("idx init . --wait; let rows = (idx find target); [($rows | where kind == file | length) ($rows | where kind == dir | length)] | to nuon")
            .expect_value_eq("[2, 1]")
    })
}

#[test]
#[serial]
fn idx_export_and_import_roundtrip() -> Result {
    Playground::setup("idx_export_and_import_roundtrip", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("searchable.txt", "hello from idx search"),
            FileWithContent("other.txt", "unrelated"),
        ]);

        test()
            .cwd(dirs.test())
            .run("idx init . --wait; idx export snapshot.json | get stored")
            .expect_value_eq(true)?;

        test()
            .cwd(dirs.test())
            .run("idx import snapshot.json | get restored")
            .expect_value_eq(true)
    })
}

#[test]
#[serial]
fn idx_import_auto_initializes_runtime_for_queries() -> Result {
    Playground::setup(
        "idx_import_auto_initializes_runtime_for_queries",
        |dirs, sandbox| {
            sandbox.with_files(&[
                FileWithContent("searchable.txt", "hello from idx import"),
                FileWithContent("other.txt", "unrelated"),
            ]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait; idx export snapshot.db | get stored")
                .expect_value_eq(true)?;

            test()
                .cwd(dirs.test())
                .run("idx drop | get dropped")
                .expect_value_eq(true)?;

            test()
                .cwd(dirs.test())
                .run("idx import snapshot.db; idx files searchable | where file_name == searchable.txt | length")
                .expect_value_eq(1)
        },
    )
}

#[test]
#[serial]
fn idx_import_restores_queryable_snapshot_when_files_are_gone() -> Result {
    Playground::setup(
        "idx_import_restores_queryable_snapshot_when_files_are_gone",
        |dirs, sandbox| {
            sandbox.with_files(&[
                FileWithContent("searchable.txt", "hello from idx import"),
                FileWithContent("other.txt", "unrelated"),
            ]);

            test()
                .cwd(dirs.test())
                .run("idx init . --wait; idx export snapshot.db | get stored")
                .expect_value_eq(true)?;

            test()
                .cwd(dirs.test())
                .run("idx drop | get dropped")
                .expect_value_eq(true)?;

            test()
                .cwd(dirs.test())
                .run("rm searchable.txt other.txt; idx import snapshot.db; idx files searchable | where file_name == searchable.txt | length")
                .expect_value_eq(1)
        },
    )
}

#[test]
#[serial]
fn idx_search_works_on_imported_snapshot() -> Result {
    Playground::setup("idx_search_works_on_imported_snapshot", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("searchable.txt", "hello from idx import"),
            FileWithContent("other.txt", "unrelated content"),
        ]);

        test()
            .cwd(dirs.test())
            .run("idx init . --wait; idx export snapshot.db | get stored")
            .expect_value_eq(true)?;

        test()
            .cwd(dirs.test())
            .run("idx drop | get dropped")
            .expect_value_eq(true)?;

        test()
            .cwd(dirs.test())
            .run("idx import snapshot.db; idx search hello | where path == searchable.txt | length")
            .expect_value_eq(1)
    })
}

#[test]
#[serial]
fn idx_search_finds_content() -> Result {
    Playground::setup("idx_search_finds_content", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContent("searchable.txt", "hello from idx search"),
            FileWithContent("other.txt", "unrelated"),
        ]);

        test()
            .cwd(dirs.test())
            .run("idx init . --wait; idx search hello | get 0.path | str contains searchable.txt")
            .expect_value_eq(true)
    })
}

#[test]
#[serial]
fn idx_drop_clears_runtime() -> Result {
    Playground::setup("idx_drop_clears_runtime", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("alpha.txt")]);

        test()
            .cwd(dirs.test())
            .run("idx init . --wait; idx drop | get dropped")
            .expect_value_eq(true)?;

        test()
            .cwd(dirs.test())
            .run("idx status | get initialized")
            .expect_value_eq(false)
    })
}
