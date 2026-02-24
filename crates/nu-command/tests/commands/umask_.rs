use std::os::unix::fs::MetadataExt;

use nix::sys::stat::{Mode, umask};
use nu_path::AbsolutePath;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

#[test]
fn mask_get() {
    Playground::setup("mask_get", |_dirs, _sandbox| {
        umask(Mode::from_bits(0o27).unwrap());

        let actual = nu!("umask");

        assert!(actual.out.contains("rwxr-x---"));
    });
}

fn get_perms(path: &AbsolutePath) -> u32 {
    path.metadata().unwrap().mode() & 0o777
}

#[test]
fn mask_set() {
    Playground::setup("mask_set", |dirs, _sandbox| {
        // Set a "baseline" mask which is different from the one set in the test
        // script, to ensure it's changed by the command.
        umask(Mode::from_bits(0o27).unwrap());

        // The umask only applies to the process setting it, so the file and
        // directory used in this test must be created inside the same script
        // which calls the umask command.
        nu!(cwd: dirs.test(), "
            umask r-x----w-;
            touch file;
            mkdir dir;
        ");

        let file_path = dirs.test().join("file");
        let dir_path = dirs.test().join("dir");

        assert!(get_perms(&file_path) == 0o402);
        assert!(get_perms(&dir_path) == 0o502);
    });
}

#[test]
fn mask_set_invalid1() {
    Playground::setup("mask_set_invalid", |_dirs, _sandbox| {
        let actual = nu!("umask invalid");

        assert!(actual.err.contains("Invalid mode"));
    });
}

#[test]
fn mask_set_invalid2() {
    Playground::setup("mask_set_invalid", |_dirs, _sandbox| {
        let actual = nu!("umask r-x");

        assert!(actual.err.contains("Invalid mode"));
    });
}

#[test]
fn mask_set_invalid3() {
    Playground::setup("mask_set_invalid", |_dirs, _sandbox| {
        let actual = nu!("umask rwxrwxrwxrwx---rwx");

        assert!(actual.err.contains("Invalid mode"));
    });
}

#[cfg(target_family = "unix")]
#[test]
fn race_overwrite_mask() {
    // See Issue #17469
    //
    // `uucore::mode::get_umask` is racy. This test verifies that our mitigation
    //  is sufficient to prevent the race.
    Playground::setup("race_overwrite_umask", |dirs, _| {
        let count = nu!(
            cwd: dirs.test(),
            "seq 0 1000 | par-each { umask } | uniq | length"
        )
        .out;
        assert_eq!(count, "1");
    });
}
