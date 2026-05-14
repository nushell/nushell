use nu_protocol::{ConfigError, Record, Type};
use nu_test_support::prelude::*;

const DEFAULT_FILES_DIR: &str = "crates/nu-utils/src/default_files";

#[track_caller]
fn config_error<const N: usize>(err: &ShellError) -> Result<&[ConfigError; N]> {
    match err {
        ShellError::InvalidConfig { errors } => match errors.as_slice().try_into() {
            Ok(errs) => Ok(errs),
            _ => panic!("expected {N} config error, got {}", errors.len()),
        },
        _ => Err(err.clone().into()),
    }
}

#[test]
fn config_is_mutable() -> Result {
    let mut tester = test();
    let () = tester.run("$env.config = { ls: { clickable_links: true } }")?;
    let () = tester.run("$env.config.ls.clickable_links = false")?;
    tester
        .run("$env.config.ls.clickable_links")
        .expect_value_eq(false)
}

#[test]
fn config_preserved_after_do() -> Result {
    let mut tester = test();
    let () = tester.run("$env.config = { ls: { clickable_links: true } }")?;
    let () = tester.run("do -i { $env.config.ls.clickable_links = false }")?;
    tester
        .run("$env.config.ls.clickable_links")
        .expect_value_eq(true)
}

#[test]
fn config_affected_when_mutated() -> Result {
    let mut tester = test();
    let () = tester.run("$env.config = { filesize: { unit: binary } }")?;
    let () = tester.run("$env.config = { filesize: { unit: metric } }")?;
    tester.run("20MB | into string").expect_value_eq("20.0 MB")
}

#[test]
fn config_affected_when_deep_mutated() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run("source default_config.nu")?;
    let () = tester.run("$env.config.filesize.unit = 'binary'")?;
    tester
        .run("20MiB | into string")
        .expect_value_eq("20.0 MiB")
}

#[test]
fn config_add_unsupported_key() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run("source default_config.nu")?;
    let shell_error = tester.run("$env.config.foo = 2").expect_shell_error()?;
    let [err] = config_error(&shell_error)?;

    match err {
        ConfigError::UnknownOption { path, .. } if path == "$env.config.foo" => Ok(()),
        _ => Err(shell_error.into()),
    }
}

#[test]
fn config_add_unsupported_type() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run("source default_config.nu")?;
    let shell_error = tester.run("$env.config.ls = '' ").expect_shell_error()?;
    let [err] = config_error(&shell_error)?;

    match err {
        ConfigError::TypeMismatch {
            expected: Type::Record(_),
            actual: Type::String,
            ..
        } => Ok(()),
        _ => Err(shell_error.into()),
    }
}

#[test]
fn config_add_unsupported_value() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run("source default_config.nu")?;
    let shell_error = tester
        .run("$env.config.history.file_format = ''")
        .expect_shell_error()?;
    let [err] = config_error(&shell_error)?;

    match err {
        ConfigError::InvalidValue { valid, actual, .. } => {
            #[cfg(feature = "sqlite")]
            assert_eq!(valid, "'sqlite' or 'plaintext'");
            #[cfg(not(feature = "sqlite"))]
            assert_eq!(valid, "'plaintext'");

            assert_eq!(actual, "''");
            Ok(())
        }
        _ => Err(shell_error.into()),
    }
}

#[test]
fn config_unsupported_key_reverted() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run("source default_config.nu")?;
    let _ = tester.run("$env.config.foo = 1").expect_shell_error()?;
    tester.run("'foo' in $env.config").expect_value_eq(false)
}

#[test]
fn config_unsupported_type_reverted() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run(" source default_config.nu")?;
    let _ = tester.run("$env.config.ls = ''").expect_shell_error()?;
    let _: Record = tester.run("$env.config.ls")?;
    Ok(())
}

#[test]
fn config_unsupported_value_reverted() -> Result {
    let mut tester = test().cwd(DEFAULT_FILES_DIR);
    let () = tester.run(" source default_config.nu")?;
    let () = tester.run("$env.config.history.file_format = 'plaintext'")?;
    let _ = tester
        .run("$env.config.history.file_format = ''")
        .expect_shell_error()?;
    tester
        .run("$env.config.history.file_format")
        .expect_value_eq("plaintext")
}
