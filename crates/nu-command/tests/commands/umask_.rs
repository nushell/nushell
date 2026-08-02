use std::os::unix::fs::MetadataExt;

use nix::sys::stat::{Mode, umask};
use nu_path::AbsolutePath;
use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn mask_get() -> Result {
    Playground::setup("mask_get", |_dirs, _sandbox| {
        umask(Mode::from_bits(0o27).unwrap());

        let actual: String = test().run("umask")?;

        assert_contains("rwxr-x---", actual);
        Ok(())
    })
}

fn get_perms(path: &AbsolutePath) -> u32 {
    path.metadata().unwrap().mode() & 0o777
}

#[test]
fn mask_set() -> Result {
    Playground::setup("mask_set", |dirs, _sandbox| {
        // Set a "baseline" mask which is different from the one set in the test
        // script, to ensure it's changed by the command.
        umask(Mode::from_bits(0o27).unwrap());

        // The umask only applies to the process setting it, so the file and
        // directory used in this test must be created inside the same script
        // which calls the umask command.
        let code = "
            umask r-x----w-;
            touch file;
            mkdir dir;
        ";
        let () = test().cwd(dirs.test()).run(code)?;

        let file_path = dirs.test().join("file");
        let dir_path = dirs.test().join("dir");

        assert_eq!(get_perms(&file_path), 0o402);
        assert_eq!(get_perms(&dir_path), 0o502);
        Ok(())
    })
}

#[test]
fn mask_set_invalid1() -> Result {
    Playground::setup("mask_set_invalid", |_dirs, _sandbox| {
        let err = test().run("umask invalid").expect_shell_error()?;

        assert_matches!(
            err,
            ShellError::Generic(err) if err.error == "Invalid mode"
        );
        Ok(())
    })
}

#[test]
fn mask_set_invalid2() -> Result {
    Playground::setup("mask_set_invalid", |_dirs, _sandbox| {
        let err = test().run("umask r-x").expect_shell_error()?;

        assert_matches!(
            err,
            ShellError::Generic(err) if err.error == "Invalid mode"
        );
        Ok(())
    })
}

#[test]
fn mask_set_invalid3() -> Result {
    Playground::setup("mask_set_invalid", |_dirs, _sandbox| {
        let err = test()
            .run("umask rwxrwxrwxrwx---rwx")
            .expect_shell_error()?;

        assert_matches!(
            err,
            ShellError::Generic(err) if err.error == "Invalid mode"
        );
        Ok(())
    })
}

#[cfg(target_family = "unix")]
#[test]
fn race_overwrite_mask() -> Result {
    // See Issue #17469
    //
    // `uucore::mode::get_umask` is racy. This test verifies that our mitigation
    //  is sufficient to prevent the race.
    Playground::setup("race_overwrite_umask", |dirs, _| {
        test()
            .cwd(dirs.test())
            .run("seq 0 1000 | par-each { umask } | uniq | length")
            .expect_value_eq(1)
    })
}
