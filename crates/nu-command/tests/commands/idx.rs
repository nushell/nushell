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
fn idx_status_reports_watch_disabled_by_default() -> Result {
    Playground::setup(
        "idx_status_reports_watch_enabled_by_default",
        |dirs, sandbox| {
            sandbox.with_files(&[EmptyFile("beta.txt")]);

            test()
                .cwd(dirs.test())
                .run("idx init .; idx status | get watch")
                .expect_value_eq(false)
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
