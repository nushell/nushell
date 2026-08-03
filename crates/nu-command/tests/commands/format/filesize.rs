use nu_test_support::fs::Stub;
use nu_test_support::prelude::*;

#[test]
fn format_filesize_without_fraction_keeps_old_output() -> Result {
    let code = "1MB | format filesize kB";
    test().run(code).expect_value_eq("1000 kB")
}

#[test]
fn format_filesize_respects_float_precision_for_fractional_values() -> Result {
    let code = "
        $env.config = ($env.config | upsert float_precision 5)
        1024B | format filesize kB
    ";

    test().run(code).expect_value_eq("1.02400 kB")
}

#[test]
fn format_filesize_with_invalid_unit() -> Result {
    let code = "1MB | format filesize sec";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}

#[test]
fn format_filesize_works() -> Result {
    Playground::setup("format_filesize_test_1", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::EmptyFile("yehuda.txt"),
            Stub::EmptyFile("jttxt"),
            Stub::EmptyFile("andres.txt"),
        ]);

        let code = "
            ls
            | format filesize kB size
            | get size
            | first
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq("0 kB")
    })
}

#[test]
fn format_filesize_works_with_nonempty_files() -> Result {
    Playground::setup(
        "format_filesize_works_with_nonempty_files",
        |dirs, sandbox| {
            sandbox.with_files(&[Stub::FileWithContentToBeTrimmed(
                "sample.toml",
                r#"
                    [dependency]
                    name = "nu"
                "#,
            )]);

            let code = "ls sample.toml | format filesize B size | get size | first";
            #[cfg(not(windows))]
            let expected = "25 B";
            #[cfg(windows)]
            let expected = "27 B";

            test().cwd(dirs.test()).run(code).expect_value_eq(expected)
        },
    )
}
