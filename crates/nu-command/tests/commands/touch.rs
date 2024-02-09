use chrono::{DateTime, Local};
use nu_test_support::fs::Stub;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);
        let path = dirs.test().join("file.txt");

        // Set file.txt's times to 0 before the test to make sure `touch` actually changes the mtime to today
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
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);
        let path = dirs.test().join("file.txt");

        // Set file.txt's times to 0 before the test to make sure `touch` actually changes the atime to today
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
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);
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
            sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);
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
        sandbox.with_files(vec![
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
