use std::{os::unix::fs::MetadataExt, path::Path};

use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
#[deps(NU)]
fn mask_get() -> Result {
    let result: CompleteResult =
        test().run_with_data("nu -n -c $in | complete", "umask rwxr-x---; umask")?;

    assert_eq!(result.exit_code, 0);
    assert_contains("rwxr-x---", result.stdout);
    assert_eq!(result.stderr, "");
    Ok(())
}

fn get_perms(path: impl AsRef<Path>) -> u32 {
    path.as_ref().metadata().unwrap().mode() & 0o777
}

#[test]
#[deps(NU)]
fn mask_set(playground: Playground) -> Result {
    // The umask only applies to the process setting it, so the file and
    // directory used in this test must be created inside the same script
    // which calls the umask command.
    let code = "
        umask r-x----w-;
        touch file;
        mkdir dir;
    ";
    let result: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data("nu -n -c $in | complete", code)?;

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stderr, "");

    let file_path = playground.path().join("file");
    let dir_path = playground.path().join("dir");

    assert_eq!(get_perms(&file_path), 0o402);
    assert_eq!(get_perms(&dir_path), 0o502);
    Ok(())
}

#[test]
fn mask_set_invalid1() -> Result {
    let err = test().run("umask invalid").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::IncorrectValue { msg, .. } if msg.starts_with("Invalid mode")
    );
    Ok(())
}

#[test]
fn mask_set_invalid2() -> Result {
    let err = test().run("umask r-x").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::IncorrectValue { msg, .. } if msg.starts_with("Invalid mode")
    );
    Ok(())
}

#[test]
fn mask_set_invalid3_() -> Result {
    let err = test()
        .run("umask rwxrwxrwxrwx---rwx")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::IncorrectValue { msg, .. } if msg.starts_with("Invalid mode")
    );
    Ok(())
}

#[cfg(target_family = "unix")]
#[test]
#[deps(NU)]
fn race_overwrite_mask() -> Result {
    // See Issue #17469
    //
    // `uucore::mode::get_umask` is racy. This test verifies that our mitigation
    //  is sufficient to prevent the race.
    let result: CompleteResult = test().run_with_data(
        "nu -n -c $in | complete",
        "seq 0 1000 | par-each { umask } | uniq | length",
    )?;

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "1");
    assert_eq!(result.stderr, "");
    Ok(())
}
