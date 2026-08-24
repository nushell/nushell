use chrono::{DateTime, Days, Local, TimeDelta, Utc};
use filetime::FileTime;
use nu_test_support::prelude::*;
use rstest::rstest;
use std::path::Path;

// Use 1 instead of 0 because 0 has a special meaning in Windows
const TIME_ONE: FileTime = FileTime::from_unix_time(1, 0);

fn file_times(file: impl AsRef<Path>) -> (FileTime, FileTime) {
    (
        file.as_ref().metadata().unwrap().accessed().unwrap().into(),
        file.as_ref().metadata().unwrap().modified().unwrap().into(),
    )
}

fn symlink_times(path: impl AsRef<Path>) -> (filetime::FileTime, filetime::FileTime) {
    let metadata = path.as_ref().symlink_metadata().unwrap();

    (
        filetime::FileTime::from_system_time(metadata.accessed().unwrap()),
        filetime::FileTime::from_system_time(metadata.modified().unwrap()),
    )
}

// From https://github.com/nushell/nushell/pull/14214
fn setup_symlink_fs(playground: &Playground) -> Result {
    playground.dir("d")?;
    playground.empty_file("f")?;
    playground.empty_file("d/f")?;
    playground.symlink("f", "fs")?;
    playground.symlink("d", "ds")?;
    playground.symlink("d/f", "fds")?;

    // sandbox.symlink does not handle symlinks to missing files well. It panics
    // But they are useful, and they should be tested.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(playground.path().join("m"), playground.path().join("fms"))
            .unwrap();
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(
            playground.path().join("m"),
            playground.path().join("fms"),
        )
        .unwrap();
    }

    // Change the file times to a known "old" value for comparison
    filetime::set_symlink_file_times(playground.path().join("f"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(playground.path().join("d"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(playground.path().join("d/f"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(playground.path().join("ds"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(playground.path().join("fs"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(playground.path().join("fds"), TIME_ONE, TIME_ONE).unwrap();
    filetime::set_symlink_file_times(playground.path().join("fms"), TIME_ONE, TIME_ONE).unwrap();
    Ok(())
}

#[test]
fn creates_a_file_when_it_doesnt_exist(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("touch i_will_be_created.txt")?;

    let path = playground.path().join("i_will_be_created.txt");
    assert!(path.exists());
    Ok(())
}

#[test]
fn creates_two_files(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch a b")?;

    let path = playground.path().join("a");
    assert!(path.exists());

    let path2 = playground.path().join("b");
    assert!(path2.exists());
    Ok(())
}

// Windows forbids file names with reserved characters
// https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
#[test]
#[cfg(not(windows))]
fn creates_a_file_when_glob_is_quoted(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch '*.txt'")?;

    let path = playground.path().join("*.txt");
    assert!(path.exists());
    Ok(())
}

#[test]
fn fails_when_glob_has_no_matches(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("touch *.txt")
        .expect_shell_error()?;

    assert_contains("No matches found for glob *.txt", err.to_string());
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[nu_test_support::test]
#[exp(nu_experimental::DC_GLOB)]
fn touch_glob_matches_when_dc_glob_enabled(
    #[ignore] playground: Playground,
    #[case] with_preexisting_files: bool,
) -> Result {
    let _sandbox_name = if with_preexisting_files {
        "touch_glob_dc_glob_preexisting"
    } else {
        "touch_glob_dc_glob_create_first"
    };

    if with_preexisting_files {
        playground.empty_file("one.txt")?;
        playground.empty_file("two.txt")?;
    } else {
        let () = test().cwd(playground.path()).run("touch one.txt two.txt")?;
    }

    let () = test().cwd(playground.path()).run("touch *.txt")?;

    assert!(playground.path().join("one.txt").exists());
    assert!(playground.path().join("two.txt").exists());
    assert!(!playground.path().join("*.txt").exists());
    Ok(())
}

#[test]
fn change_modified_time_of_file_to_today(playground: Playground) -> Result {
    playground.empty_file("file.txt")?;
    let path = playground.path().join("file.txt");

    // Set file.txt's times to the past before the test to make sure `touch` actually changes the mtime to today
    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -m file.txt")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();

    assert_eq!(today, mtime_day);

    // Check that atime remains unchanged
    assert_eq!(
        TIME_ONE,
        FileTime::from_system_time(metadata.accessed().unwrap())
    );
    Ok(())
}

#[test]
fn change_access_time_of_file_to_today(playground: Playground) -> Result {
    playground.empty_file("file.txt")?;
    let path = playground.path().join("file.txt");

    // Set file.txt's times to the past before the test to make sure `touch` actually changes the atime to today
    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -a file.txt")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

    assert_eq!(today, atime_day);

    // Check that mtime remains unchanged
    assert_eq!(
        TIME_ONE,
        FileTime::from_system_time(metadata.modified().unwrap())
    );
    Ok(())
}

#[test]
fn change_modified_and_access_time_of_file_to_today(playground: Playground) -> Result {
    playground.empty_file("file.txt")?;
    let path = playground.path().join("file.txt");

    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -a -m file.txt")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
    let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

    assert_eq!(today, mtime_day);
    assert_eq!(today, atime_day);
    Ok(())
}

#[test]
fn change_modified_and_access_time_of_files_matching_glob_to_today(
    playground: Playground,
) -> Result {
    playground.empty_file("file.txt")?;

    let path = playground.path().join("file.txt");
    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch *.txt")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
    let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

    assert_eq!(today, mtime_day);
    assert_eq!(today, atime_day);
    Ok(())
}

#[test]
fn not_create_file_if_it_not_exists(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch -c file.txt")?;

    let path = playground.path().join("file.txt");

    assert!(!path.exists());
    Ok(())
}

#[test]
fn change_file_times_if_exists_with_no_create(playground: Playground) -> Result {
    playground.empty_file("file.txt")?;
    let path = playground.path().join("file.txt");

    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -c file.txt")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
    let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

    assert_eq!(today, mtime_day);
    assert_eq!(today, atime_day);
    Ok(())
}

#[test]
fn creates_file_three_dots(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch file...")?;

    let path = playground.path().join("file...");
    assert!(path.exists());
    Ok(())
}

#[test]
fn creates_file_four_dots(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch file....")?;

    let path = playground.path().join("file....");
    assert!(path.exists());
    Ok(())
}

#[test]
fn creates_file_four_dots_quotation_marks(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch 'file....'")?;

    let path = playground.path().join("file....");
    assert!(path.exists());
    Ok(())
}

#[test]
fn change_file_times_to_reference_file(playground: Playground) -> Result {
    playground.empty_file("reference_file")?;
    playground.empty_file("target_file")?;

    let reference = playground.path().join("reference_file");
    let target = playground.path().join("target_file");

    // Change the times for reference
    filetime::set_file_times(&reference, FileTime::from_unix_time(1337, 0), TIME_ONE).unwrap();

    // target should have today's date since it was just created, but reference should be different
    assert_ne!(
        reference.metadata().unwrap().accessed().unwrap(),
        target.metadata().unwrap().accessed().unwrap()
    );
    assert_ne!(
        reference.metadata().unwrap().modified().unwrap(),
        target.metadata().unwrap().modified().unwrap()
    );

    let () = test()
        .cwd(playground.path())
        .run("touch -r reference_file target_file")?;

    assert_eq!(
        reference.metadata().unwrap().accessed().unwrap(),
        target.metadata().unwrap().accessed().unwrap()
    );
    assert_eq!(
        reference.metadata().unwrap().modified().unwrap(),
        target.metadata().unwrap().modified().unwrap()
    );
    Ok(())
}

#[test]
fn change_file_mtime_to_reference(playground: Playground) -> Result {
    playground.empty_file("reference_file")?;
    playground.empty_file("target_file")?;

    let reference = playground.path().join("reference_file");
    let target = playground.path().join("target_file");

    // Change the times for reference
    filetime::set_file_times(&reference, TIME_ONE, FileTime::from_unix_time(1337, 0)).unwrap();

    // target should have today's date since it was just created, but reference should be different
    assert_ne!(file_times(&reference), file_times(&target));

    // Save target's current atime to make sure it is preserved
    let target_original_atime = target.metadata().unwrap().accessed().unwrap();

    let () = test()
        .cwd(playground.path())
        .run("touch -mr reference_file target_file")?;

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
    Ok(())
}

// TODO when https://github.com/uutils/coreutils/issues/6629 is fixed,
// unignore this test
#[test]
#[ignore]
fn change_file_times_to_reference_file_with_date(playground: Playground) -> Result {
    playground.empty_file("reference_file")?;
    playground.empty_file("target_file")?;

    let reference = playground.path().join("reference_file");
    let target = playground.path().join("target_file");

    let now = Utc::now();

    let ref_atime = now;
    let ref_mtime = now.checked_sub_days(Days::new(5)).unwrap();

    // Change the times for reference
    filetime::set_file_times(
        reference,
        FileTime::from_unix_time(ref_atime.timestamp(), ref_atime.timestamp_subsec_nanos()),
        FileTime::from_unix_time(ref_mtime.timestamp(), ref_mtime.timestamp_subsec_nanos()),
    )
    .unwrap();

    let () = test()
        .cwd(playground.path())
        .run(r#"touch -r reference_file -d "yesterday" target_file"#)?;

    let (got_atime, got_mtime) = file_times(target);
    let got = (
        DateTime::from_timestamp(got_atime.seconds(), got_atime.nanoseconds()).unwrap(),
        DateTime::from_timestamp(got_mtime.seconds(), got_mtime.nanoseconds()).unwrap(),
    );
    assert_eq!(
        (
            now.checked_sub_days(Days::new(1)).unwrap(),
            now.checked_sub_days(Days::new(6)).unwrap()
        ),
        got
    );
    Ok(())
}

#[test]
fn change_file_times_to_timestamp(playground: Playground) -> Result {
    playground.empty_file("target_file")?;

    let target = playground.path().join("target_file");
    let timestamp = DateTime::from_timestamp(TIME_ONE.unix_seconds(), TIME_ONE.nanoseconds())
        .unwrap()
        .to_rfc3339();

    let () = test()
        .cwd(playground.path())
        .run(format!("touch --timestamp {} target_file", timestamp))?;

    assert_eq!((TIME_ONE, TIME_ONE), file_times(target));
    Ok(())
}

#[test]
fn change_modified_time_of_dir_to_today(playground: Playground) -> Result {
    playground.dir("test_dir")?;
    let path = playground.path().join("test_dir");

    filetime::set_file_mtime(&path, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -m test_dir")?;

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day =
        DateTime::<Local>::from(path.metadata().unwrap().modified().unwrap()).date_naive();

    assert_eq!(today, mtime_day);
    Ok(())
}

#[test]
fn change_access_time_of_dir_to_today(playground: Playground) -> Result {
    playground.dir("test_dir")?;
    let path = playground.path().join("test_dir");

    filetime::set_file_atime(&path, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -a test_dir")?;

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let atime_day =
        DateTime::<Local>::from(path.metadata().unwrap().accessed().unwrap()).date_naive();

    assert_eq!(today, atime_day);
    Ok(())
}

#[test]
fn change_modified_and_access_time_of_dir_to_today(playground: Playground) -> Result {
    playground.dir("test_dir")?;
    let path = playground.path().join("test_dir");

    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch -a -m test_dir")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
    let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

    assert_eq!(today, mtime_day);
    assert_eq!(today, atime_day);
    Ok(())
}

// TODO when https://github.com/uutils/coreutils/issues/6629 is fixed,
// unignore this test
#[test]
#[ignore]
fn change_file_times_to_date(playground: Playground) -> Result {
    playground.empty_file("target_file")?;

    let expected = Utc::now().checked_sub_signed(TimeDelta::hours(2)).unwrap();
    let () = test()
        .cwd(playground.path())
        .run("touch -d '-2 hours' target_file")?;

    let (got_atime, got_mtime) = file_times(playground.path().join("target_file"));
    let got_atime = DateTime::from_timestamp(got_atime.seconds(), got_atime.nanoseconds()).unwrap();
    let got_mtime = DateTime::from_timestamp(got_mtime.seconds(), got_mtime.nanoseconds()).unwrap();
    let threshold = TimeDelta::minutes(1);
    assert!(
        got_atime.signed_duration_since(expected).lt(&threshold)
            && got_mtime.signed_duration_since(expected).lt(&threshold),
        "Expected: {expected}. Got: atime={got_atime}, mtime={got_mtime}"
    );
    assert!(got_mtime.signed_duration_since(expected).lt(&threshold));
    Ok(())
}

#[test]
fn change_dir_three_dots_times(playground: Playground) -> Result {
    playground.dir("test_dir...")?;
    let path = playground.path().join("test_dir...");

    filetime::set_file_times(&path, TIME_ONE, TIME_ONE).unwrap();

    let () = test().cwd(playground.path()).run("touch test_dir...")?;

    let metadata = path.metadata().unwrap();

    // Check only the date since the time may not match exactly
    let today = Local::now().date_naive();
    let mtime_day = DateTime::<Local>::from(metadata.modified().unwrap()).date_naive();
    let atime_day = DateTime::<Local>::from(metadata.accessed().unwrap()).date_naive();

    assert_eq!(today, mtime_day);
    assert_eq!(today, atime_day);
    Ok(())
}

#[test]
fn change_dir_times_to_reference_dir(playground: Playground) -> Result {
    playground.dir("reference_dir")?;
    playground.dir("target_dir")?;

    let reference = playground.path().join("reference_dir");
    let target = playground.path().join("target_dir");

    // Change the times for reference
    filetime::set_file_times(&reference, FileTime::from_unix_time(1337, 0), TIME_ONE).unwrap();

    // target should have today's date since it was just created, but reference should be different
    assert_ne!(
        reference.metadata().unwrap().accessed().unwrap(),
        target.metadata().unwrap().accessed().unwrap()
    );
    assert_ne!(
        reference.metadata().unwrap().modified().unwrap(),
        target.metadata().unwrap().modified().unwrap()
    );

    let () = test()
        .cwd(playground.path())
        .run("touch -r reference_dir target_dir")?;

    assert_eq!(
        reference.metadata().unwrap().accessed().unwrap(),
        target.metadata().unwrap().accessed().unwrap()
    );
    assert_eq!(
        reference.metadata().unwrap().modified().unwrap(),
        target.metadata().unwrap().modified().unwrap()
    );
    Ok(())
}

#[test]
fn change_dir_atime_to_reference(playground: Playground) -> Result {
    playground.dir("reference_dir")?;
    playground.dir("target_dir")?;

    let reference = playground.path().join("reference_dir");
    let target = playground.path().join("target_dir");

    // Change the times for reference
    filetime::set_file_times(&reference, FileTime::from_unix_time(1337, 0), TIME_ONE).unwrap();

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

    let () = test()
        .cwd(playground.path())
        .run("touch -ar reference_dir target_dir")?;

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
    Ok(())
}

#[test]
fn create_a_file_with_tilde(playground: Playground) -> Result {
    let () = test().cwd(playground.path()).run("touch '~tilde'")?;
    assert!(playground.path().join("~tilde").exists());

    // pass variable
    let () = test()
        .cwd(playground.path())
        .run("let f = '~tilde2'; touch $f")?;
    assert!(playground.path().join("~tilde2").exists());
    Ok(())
}

#[test]
fn respects_cwd(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mkdir 'dir'; cd 'dir'; touch 'i_will_be_created.txt'")?;

    let path = playground.path().join("dir/i_will_be_created.txt");
    assert!(path.exists());
    Ok(())
}

#[test]
fn reference_respects_cwd(playground: Playground) -> Result {
    let () = test()
        .cwd(playground.path())
        .run("mkdir 'dir'; cd 'dir'; touch 'ref.txt'; touch --reference 'ref.txt' 'foo.txt'")?;

    let path = playground.path().join("dir/foo.txt");
    assert!(path.exists());
    Ok(())
}

#[test]
#[deps(NU)]
fn recognizes_stdout(playground: Playground) -> Result {
    let _: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data("let code; nu -n -c $code | complete", "touch -")?;

    assert!(!playground.path().join("-").exists());
    Ok(())
}

#[test]
fn follow_symlinks(playground: Playground) -> Result {
    setup_symlink_fs(&playground)?;

    let missing = playground.path().join("m");
    assert!(!missing.exists());

    let code = "
        touch fds
        touch ds
        touch fs
        touch fms
    ";
    let () = test().cwd(playground.path()).run(code)?;

    // We created the missing symlink target
    assert!(missing.exists());

    // The timestamps for files and directories were changed from TIME_ONE
    let file_times = symlink_times(playground.path().join("f"));
    let dir_times = symlink_times(playground.path().join("d"));
    let dir_file_times = symlink_times(playground.path().join("d/f"));

    assert_ne!(file_times, (TIME_ONE, TIME_ONE));
    assert_ne!(dir_times, (TIME_ONE, TIME_ONE));
    assert_ne!(dir_file_times, (TIME_ONE, TIME_ONE));

    // For symlinks, they remain (mostly) the same
    // We can't test accessed times, since to reach the target file, the symlink must be accessed!
    let file_symlink_times = symlink_times(playground.path().join("fs"));
    let dir_symlink_times = symlink_times(playground.path().join("ds"));
    let dir_file_symlink_times = symlink_times(playground.path().join("fds"));
    let file_missing_symlink_times = symlink_times(playground.path().join("fms"));

    assert_eq!(file_symlink_times.1, TIME_ONE);
    assert_eq!(dir_symlink_times.1, TIME_ONE);
    assert_eq!(dir_file_symlink_times.1, TIME_ONE);
    assert_eq!(file_missing_symlink_times.1, TIME_ONE);
    Ok(())
}

#[test]
fn no_follow_symlinks(playground: Playground) -> Result {
    setup_symlink_fs(&playground)?;

    let missing = playground.path().join("m");
    assert!(!missing.exists());

    let code = "
        touch fds -s
        touch ds -s
        touch fs -s
        touch fms -s
    ";
    let () = test().cwd(playground.path()).run(code)?;

    // We did not create the missing symlink target
    assert!(!missing.exists());

    // The timestamps for files and directories remain the same
    let file_times = symlink_times(playground.path().join("f"));
    let dir_times = symlink_times(playground.path().join("d"));
    let dir_file_times = symlink_times(playground.path().join("d/f"));

    assert_eq!(file_times, (TIME_ONE, TIME_ONE));
    assert_eq!(dir_times, (TIME_ONE, TIME_ONE));
    assert_eq!(dir_file_times, (TIME_ONE, TIME_ONE));

    // For symlinks, everything changed. (except their targets, and paths, and personality)
    let file_symlink_times = symlink_times(playground.path().join("fs"));
    let dir_symlink_times = symlink_times(playground.path().join("ds"));
    let dir_file_symlink_times = symlink_times(playground.path().join("fds"));
    let file_missing_symlink_times = symlink_times(playground.path().join("fms"));

    assert_ne!(file_symlink_times, (TIME_ONE, TIME_ONE));
    assert_ne!(dir_symlink_times, (TIME_ONE, TIME_ONE));
    assert_ne!(dir_file_symlink_times, (TIME_ONE, TIME_ONE));
    assert_ne!(file_missing_symlink_times, (TIME_ONE, TIME_ONE));
    Ok(())
}
