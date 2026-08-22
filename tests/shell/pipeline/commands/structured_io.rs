use nu_experimental::STRUCTURED_IO;
use nu_test_support::{fs::Stub::FileWithContent, prelude::*};

#[test]
#[deps(NU)]
fn child_nu_output_is_raw_without_structured_io() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | describe")
        .expect_value_eq("byte stream")
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn child_nu_list_stays_structured() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn child_nu_list_is_usable() -> Result {
    test()
        .run("nu -n -c '[1 2 3]' | math sum")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn child_nu_table_columns() -> Result {
    test()
        .run("nu -n -c '[[a b]; [1 2] [3 4]]' | columns")
        .expect_value_eq(["a", "b"])
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn parent_sends_structured_stdin() -> Result {
    test()
        .run("[1 2 3] | nu -n -c '$in | math sum'")
        .expect_value_eq(6)
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn parent_sends_table_stdin() -> Result {
    test()
        .run("[[name]; [foo] [bar]] | nu -n -c '$in | get name'")
        .expect_value_eq(["foo", "bar"])
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn print_does_not_corrupt_structured_output() -> Result {
    test()
        .run("nu -n -c 'print hello; [1 2 3]' | describe")
        .expect_value_eq("list<int>")
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn child_script_file_stays_structured() -> Result {
    Playground::setup("structured_io_script_file", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            r#"
                def main [] {
                    [1 2 3]
                }
            "#,
        )]);

        test()
            .cwd(dirs.test())
            .run("nu -n foo.nu | math sum")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
#[cfg(unix)]
fn shebang_script_stays_structured() -> Result {
    use std::os::unix::fs::PermissionsExt;

    Playground::setup("structured_io_shebang", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "foo.nu",
            r#"#!/usr/bin/env nu
def main [] {
    [1 2 3]
}
"#,
        )]);

        let script = dirs.test().join("foo.nu");
        let mut permissions = std::fs::metadata(&script)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions)?;

        test()
            .cwd(dirs.test())
            .run("./foo.nu | math sum")
            .expect_value_eq(6)
    })
}

#[test]
#[deps(NU)]
#[exp(STRUCTURED_IO)]
fn unserializable_closure_errors() -> Result {
    test()
        .run("nu -n -c '{|x| $x}'")
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}
