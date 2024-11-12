use chrono::{DateTime, Local};
use nu_test_support::fs::{files_exist_at, Stub};
use nu_test_support::nu;
use nu_test_support::playground::{Dirs, Playground};

// Use 1 instead of 0 because 0 has a special meaning in Windows
const TIME_ONE: filetime::FileTime = filetime::FileTime::from_unix_time(1, 0);

#[test]
fn creates_a_file_when_it_doesnt_exist() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch i_will_be_created.txt"
        );

        let path = dirs.test().join("i_will_be_created.txt");
        assert!(path.exists());
    })
}

#[test]
fn creates_two_files() {
    Playground::setup("create_test_2", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch a b"
        );

        let path = dirs.test().join("a");
        assert!(path.exists());

        let path2 = dirs.test().join("b");
        assert!(path2.exists());
    })
}

#[test]
fn change_modified_time_of_file_to_today() {
    Playground::setup("change_time_test_9", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("file.txt")]);
        let path = dirs.test().join("file.txt");

        // Set file.txt's times to the past before the test to make sure `touch` actually changes the mtime to today
        filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -m file.txt"
        );

        let metadata = path.metadata().unwrap();

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();

        assert_eq!(today, mtime_day);

        // Check that atime remains unchanged
        assert_eq!(
            TIME_ONE,
            filetime::FileTime::from_system_time(metadata.accessed().unwrap())
        );
    })
}

#[test]
fn change_access_time_of_file_to_today() {
    Playground::setup("change_time_test_18", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("file.txt")]);
        let path = dirs.test().join("file.txt");

        // Set file.txt's times to the past before the test to make sure `touch` actually changes the atime to today
        filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -a file.txt"
        );

        let metadata = path.metadata().unwrap();

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

        assert_eq!(today, atime_day);

        // Check that mtime remains unchanged
        assert_eq!(
            TIME_ONE,
            filetime::FileTime::from_system_time(metadata.modified().unwrap())
        );
    })
}

#[test]
fn change_modified_and_access_time_of_file_to_today() {
    Playground::setup("change_time_test_27", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("file.txt")]);
        let path = dirs.test().join("file.txt");

        filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -a -m file.txt"
        );

        let metadata = path.metadata().unwrap();

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
        let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

        assert_eq!(today, mtime_day);
        assert_eq!(today, atime_day);
    })
}

#[test]
fn not_create_file_if_it_not_exists() {
    Playground::setup("change_time_test_28", |dirs, _sandbox| {
        let outcome = nu!(
            cwd: dirs.test(),
            "touch -c file.txt"
        );

        let path = dirs.test().join("file.txt");

        assert!(!path.exists());

        // If --no-create is improperly handled `touch` may error when trying to change the times of a nonexistent file
        assert!(outcome.status.success())
    })
}

#[test]
fn change_file_times_if_exists_with_no_create() {
    Playground::setup(
        "change_file_times_if_exists_with_no_create",
        |dirs, sandbox| {
            sandbox.with_files(&[Stub::EmptyFile("file.txt")]);
            let path = dirs.test().join("file.txt");

            filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

            nu!(
                cwd: dirs.test(),
                "touch -c file.txt"
            );

            let metadata = path.metadata().unwrap();

            // Check only the date since the time may not match exactly
            let today = Local::now().date_naive();
            let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
            let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

            assert_eq!(today, mtime_day);
            assert_eq!(today, atime_day);
        },
    )
}

#[test]
fn creates_file_three_dots() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch file..."
        );

        let path = dirs.test().join("file...");
        assert!(path.exists());
    })
}

#[test]
fn creates_file_four_dots() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch file...."
        );

        let path = dirs.test().join("file....");
        assert!(path.exists());
    })
}

#[test]
fn creates_file_four_dots_quotation_marks() {
    Playground::setup("create_test_1", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch 'file....'"
        );

        let path = dirs.test().join("file....");
        assert!(path.exists());
    })
}

#[test]
fn change_file_times_to_reference_file() {
    Playground::setup("change_dir_times_to_reference_dir", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::EmptyFile("reference_file"),
            Stub::EmptyFile("target_file"),
        ]);

        let reference = dirs.test().join("reference_file");
        let target = dirs.test().join("target_file");

        // Change the times for reference
        filetime::set_file_times(
            &reference,
            filetime::FileTime::from_unix_time(1337, 0),
            TIME_ONE,
        )
        .unwrap();

        // target should have today's date since it was just created, but reference should be different
        assert_ne!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_ne!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );

        nu!(
            cwd: dirs.test(),
            "touch -r reference_file target_file"
        );

        assert_eq!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_eq!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );
    })
}

#[test]
fn change_file_mtime_to_reference() {
    Playground::setup("change_file_mtime_to_reference", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::EmptyFile("reference_file"),
            Stub::EmptyFile("target_file"),
        ]);

        let reference = dirs.test().join("reference_file");
        let target = dirs.test().join("target_file");

        // Change the times for reference
        filetime::set_file_times(
            &reference,
            TIME_ONE,
            filetime::FileTime::from_unix_time(1337, 0),
        )
        .unwrap();

        // target should have today's date since it was just created, but reference should be different
        assert_ne!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_ne!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );

        // Save target's current atime to make sure it is preserved
        let target_original_atime = target.metadata().unwrap().accessed().unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -mr reference_file target_file"
        );

        assert_eq!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );
        assert_ne!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_eq!(
            target_original_atime,
            target.metadata().unwrap().accessed().unwrap()
        );
    })
}

#[test]
fn change_modified_time_of_dir_to_today() {
    Playground::setup("change_dir_mtime", |dirs, sandbox| {
        sandbox.mkdir("test_dir");
        let path = dirs.test().join("test_dir");

        filetime::set_file_mtime(&path, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -m test_dir"
        );

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let mtime_day =
            DateTime::<Local>::from(path.metadata().unwrap().modified().unwrap()).date_naive();

        assert_eq!(today, mtime_day);
    })
}

#[test]
fn change_access_time_of_dir_to_today() {
    Playground::setup("change_dir_atime", |dirs, sandbox| {
        sandbox.mkdir("test_dir");
        let path = dirs.test().join("test_dir");

        filetime::set_file_atime(&path, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -a test_dir"
        );

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let atime_day =
            DateTime::<Local>::from(path.metadata().unwrap().accessed().unwrap()).date_naive();

        assert_eq!(today, atime_day);
    })
}

#[test]
fn change_modified_and_access_time_of_dir_to_today() {
    Playground::setup("change_dir_times", |dirs, sandbox| {
        sandbox.mkdir("test_dir");
        let path = dirs.test().join("test_dir");

        filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -a -m test_dir"
        );

        let metadata = path.metadata().unwrap();

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
        let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

        assert_eq!(today, mtime_day);
        assert_eq!(today, atime_day);
    })
}

#[test]
fn change_dir_three_dots_times() {
    Playground::setup("change_dir_three_dots_times", |dirs, sandbox| {
        sandbox.mkdir("test_dir...");
        let path = dirs.test().join("test_dir...");

        filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch test_dir..."
        );

        let metadata = path.metadata().unwrap();

        // Check only the date since the time may not match exactly
        let today = Local::now().date_naive();
        let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
        let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

        assert_eq!(today, mtime_day);
        assert_eq!(today, atime_day);
    })
}

#[test]
fn change_dir_times_to_reference_dir() {
    Playground::setup("change_dir_times_to_reference_dir", |dirs, sandbox| {
        sandbox.mkdir("reference_dir");
        sandbox.mkdir("target_dir");

        let reference = dirs.test().join("reference_dir");
        let target = dirs.test().join("target_dir");

        // Change the times for reference
        filetime::set_file_times(
            &reference,
            filetime::FileTime::from_unix_time(1337, 0),
            TIME_ONE,
        )
        .unwrap();

        // target should have today's date since it was just created, but reference should be different
        assert_ne!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_ne!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );

        nu!(
            cwd: dirs.test(),
            "touch -r reference_dir target_dir"
        );

        assert_eq!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_eq!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );
    })
}

#[test]
fn change_dir_atime_to_reference() {
    Playground::setup("change_dir_atime_to_reference", |dirs, sandbox| {
        sandbox.mkdir("reference_dir");
        sandbox.mkdir("target_dir");

        let reference = dirs.test().join("reference_dir");
        let target = dirs.test().join("target_dir");

        // Change the times for reference
        filetime::set_file_times(
            &reference,
            filetime::FileTime::from_unix_time(1337, 0),
            TIME_ONE,
        )
        .unwrap();

        // target should have today's date since it was just created, but reference should be different
        assert_ne!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_ne!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );

        // Save target's current mtime to make sure it is preserved
        let target_original_mtime = target.metadata().unwrap().modified().unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -ar reference_dir target_dir"
        );

        assert_eq!(
            reference.metadata().unwrap().accessed().unwrap(),
            target.metadata().unwrap().accessed().unwrap()
        );
        assert_ne!(
            reference.metadata().unwrap().modified().unwrap(),
            target.metadata().unwrap().modified().unwrap()
        );
        assert_eq!(
            target_original_mtime,
            target.metadata().unwrap().modified().unwrap()
        );
    })
}

#[test]
fn create_a_file_with_tilde() {
    Playground::setup("touch with tilde", |dirs, _| {
        let actual = nu!(cwd: dirs.test(), "touch '~tilde'");
        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["~tilde"], dirs.test()));

        // pass variable
        let actual = nu!(cwd: dirs.test(), "let f = '~tilde2'; touch $f");
        assert!(actual.err.is_empty());
        assert!(files_exist_at(&["~tilde2"], dirs.test()));
    })
}

#[test]
fn respects_cwd() {
    Playground::setup("touch_respects_cwd", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "mkdir 'dir'; cd 'dir'; touch 'i_will_be_created.txt'"
        );

        let path = dirs.test().join("dir/i_will_be_created.txt");
        assert!(path.exists());
    })
}

#[test]
fn reference_respects_cwd() {
    Playground::setup("touch_reference_respects_cwd", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "mkdir 'dir'; cd 'dir'; touch 'ref.txt'; touch --reference 'ref.txt' 'foo.txt'"
        );

        let path = dirs.test().join("dir/foo.txt");
        assert!(path.exists());
    })
}

fn setup_symlink_fs(dirs: &Dirs, sandbox: &mut Playground<'_>) {
    sandbox.mkdir("d");
    sandbox.with_files(&[Stub::EmptyFile("f"), Stub::EmptyFile("d/f")]);
    sandbox.symlink("f", "fs");
    sandbox.symlink("d", "ds");
    sandbox.symlink("d/f", "fds");

    // sandbox.symlink does not handle symlinks to missing files well. It panics
    // But they are useful, and they should be tested.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(dirs.test().join("m"), dirs.test().join("fms")).unwrap();
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(dirs.test().join("m"), dirs.test().join("fms")).unwrap();
    }

    // Change the file times to a known "old" value for comparison
    filetime::set_symlink_file_times(dirs.test().join("f"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(dirs.test().join("d"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(dirs.test().join("d/f"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(dirs.test().join("ds"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(dirs.test().join("fs"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(dirs.test().join("fds"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(dirs.test().join("fms"), TIME_ONE, TIME_ONE).unwrap();
}

fn get_times(path: &nu_path::AbsolutePath) -> (filetime::FileTime, filetime::FileTime) {
    let metadata = path.symlink_metadata().unwrap();

    (
        filetime::FileTime::from_system_time(metadata.accessed().unwrap()),
        filetime::FileTime::from_system_time(metadata.modified().unwrap()),
    )
}

#[test]
fn follow_symlinks() {
    Playground::setup("touch_follows_symlinks", |dirs, sandbox| {
        setup_symlink_fs(&dirs, sandbox);

        let missing = dirs.test().join("m");
        assert!(!missing.exists());

        nu!(
            cwd: dirs.test(),
            "
                touch fds
                touch ds
                touch fs
                touch fms
            "
        );

        // We created the missing symlink target
        assert!(missing.exists());

        // The timestamps for files and directories were changed from TIME_ONE
        let file_times = get_times(&dirs.test().join("f"));
        let dir_times = get_times(&dirs.test().join("d"));
        let dir_file_times = get_times(&dirs.test().join("d/f"));

        assert_ne!(file_times, (TIME_ONE, TIME_ONE));
        assert_ne!(dir_times, (TIME_ONE, TIME_ONE));
        assert_ne!(dir_file_times, (TIME_ONE, TIME_ONE));

        // For symlinks, they remain (mostly) the same
        // We can't test accessed times, since to reach the target file, the symlink must be accessed!
        let file_symlink_times = get_times(&dirs.test().join("fs"));
        let dir_symlink_times = get_times(&dirs.test().join("ds"));
        let dir_file_symlink_times = get_times(&dirs.test().join("fds"));
        let file_missing_symlink_times = get_times(&dirs.test().join("fms"));

        assert_eq!(file_symlink_times.1, TIME_ONE);
        assert_eq!(dir_symlink_times.1, TIME_ONE);
        assert_eq!(dir_file_symlink_times.1, TIME_ONE);
        assert_eq!(file_missing_symlink_times.1, TIME_ONE);
    })
}

#[test]
fn no_follow_symlinks() {
    Playground::setup("touch_touches_symlinks", |dirs, sandbox| {
        setup_symlink_fs(&dirs, sandbox);

        let missing = dirs.test().join("m");
        assert!(!missing.exists());

        nu!(
            cwd: dirs.test(),
            "
                touch fds -s
                touch ds -s
                touch fs -s
                touch fms -s
            "
        );

        // We did not create the missing symlink target
        assert!(!missing.exists());

        // The timestamps for files and directories remain the same
        let file_times = get_times(&dirs.test().join("f"));
        let dir_times = get_times(&dirs.test().join("d"));
        let dir_file_times = get_times(&dirs.test().join("d/f"));

        assert_eq!(file_times, (TIME_ONE, TIME_ONE));
        assert_eq!(dir_times, (TIME_ONE, TIME_ONE));
        assert_eq!(dir_file_times, (TIME_ONE, TIME_ONE));

        // For symlinks, everything changed. (except their targets, and paths, and personality)
        let file_symlink_times = get_times(&dirs.test().join("fs"));
        let dir_symlink_times = get_times(&dirs.test().join("ds"));
        let dir_file_symlink_times = get_times(&dirs.test().join("fds"));
        let file_missing_symlink_times = get_times(&dirs.test().join("fms"));

        assert_ne!(file_symlink_times, (TIME_ONE, TIME_ONE));
        assert_ne!(dir_symlink_times, (TIME_ONE, TIME_ONE));
        assert_ne!(dir_file_symlink_times, (TIME_ONE, TIME_ONE));
        assert_ne!(file_missing_symlink_times, (TIME_ONE, TIME_ONE));
    })
}
