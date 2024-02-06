use chrono::{DateTime, Local};
use nu_test_support::fs::Stub;
use nu_test_support::nu;
use nu_test_support::playground::Playground;

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

        nu!(
            cwd: dirs.test(),
            "touch -m file.txt"
        );

        let path = dirs.test().join("file.txt");

        // Check only the date since the time may not match exactly
        let date = Local::now().date_naive();
        let actual_date_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());
        let actual_date = actual_date_time.date_naive();

        assert_eq!(date, actual_date);
    })
}

#[test]
fn change_access_time_of_file_to_today() {
    Playground::setup("change_time_test_18", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a file.txt"
        );

        let path = dirs.test().join("file.txt");

        // Check only the date since the time may not match exactly
        let date = Local::now().date_naive();
        let actual_date_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());
        let actual_date = actual_date_time.date_naive();

        assert_eq!(date, actual_date);
    })
}

#[test]
fn change_modified_and_access_time_of_file_to_today() {
    Playground::setup("change_time_test_27", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a -m file.txt"
        );

        let metadata = dirs.test().join("file.txt").metadata().unwrap();

        // Check only the date since the time may not match exactly
        let date = Local::now().date_naive();
        let adate_time: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let adate = adate_time.date_naive();
        let mdate_time: DateTime<Local> = DateTime::from(metadata.modified().unwrap());
        let mdate = mdate_time.date_naive();

        assert_eq!(date, adate);
        assert_eq!(date, mdate);
    })
}

#[test]
fn not_create_file_if_it_not_exists() {
    Playground::setup("change_time_test_28", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch -c file.txt"
        );

        let path = dirs.test().join("file.txt");

        assert!(!path.exists());

        nu!(
            cwd: dirs.test(),
            "touch -c file.txt"
        );

        let path = dirs.test().join("file.txt");

        assert!(!path.exists());
    })
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
fn change_modified_time_of_dir_to_today() {
    Playground::setup("change_dir_mtime", |dirs, sandbox| {
        sandbox.mkdir("test_dir");

        let path = dirs.test().join("test_dir");

        // Set test_dir's mtime to 0 before the test to make sure `touch` actually changes the mtime to today
        filetime::set_file_mtime(&path, filetime::FileTime::zero()).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -m test_dir"
        );

        // Check only the date since the time may not match exactly
        let date = Local::now().date_naive();
        let actual_date_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());
        let actual_date = actual_date_time.date_naive();

        assert_eq!(date, actual_date);
    })
}

#[test]
fn change_access_time_of_dir_to_today() {
    Playground::setup("change_dir_atime", |dirs, sandbox| {
        sandbox.mkdir("test_dir");

        let path = dirs.test().join("test_dir");

        filetime::set_file_atime(&path, filetime::FileTime::zero()).unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -a test_dir"
        );

        // Check only the date since the time may not match exactly
        let date = Local::now().date_naive();
        let actual_date_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());
        let actual_date = actual_date_time.date_naive();

        assert_eq!(date, actual_date);
    })
}

#[test]
fn change_modified_and_access_time_of_dir_to_today() {
    Playground::setup("change_dir_times", |dirs, sandbox| {
        sandbox.mkdir("test_dir");

        let path = dirs.test().join("test_dir");

        filetime::set_file_times(
            &path,
            filetime::FileTime::zero(),
            filetime::FileTime::zero(),
        )
        .unwrap();

        nu!(
            cwd: dirs.test(),
            "touch -a -m test_dir"
        );

        let metadata = path.metadata().unwrap();

        // Check only the date since the time may not match exactly
        let date = Local::now().date_naive();
        let adate_time: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let adate = adate_time.date_naive();
        let mdate_time: DateTime<Local> = DateTime::from(metadata.modified().unwrap());
        let mdate = mdate_time.date_naive();

        assert_eq!(date, adate);
        assert_eq!(date, mdate);
    })
}

#[test]
fn change_dir_three_dots_times() {
    Playground::setup("change_dir_three_dots_times", |dirs, sandbox| {
        sandbox.mkdir("test_dir...");

        let path = dirs.test().join("test_dir...");

        filetime::set_file_times(
            &path,
            filetime::FileTime::zero(),
            filetime::FileTime::zero(),
        )
        .unwrap();

        nu!(
            cwd: dirs.test(),
            "touch test_dir..."
        );

        let metadata = path.metadata().unwrap();

        let date = Local::now().date_naive();
        let adate_time: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let adate = adate_time.date_naive();
        let mdate_time: DateTime<Local> = DateTime::from(metadata.modified().unwrap());
        let mdate = mdate_time.date_naive();

        assert_eq!(date, adate);
        assert_eq!(date, mdate);
    })
}

#[test]
fn change_dir_times_to_reference_dir() {
    Playground::setup("change_dir_times_to_reference_dir", |dirs, sandbox| {
        sandbox.mkdir("reference_dir");
        sandbox.mkdir("target_dir");

        let reference_dir_path = dirs.test().join("reference_dir");
        let target_dir_path = dirs.test().join("target_dir");

        // Change the times for reference_dir
        filetime::set_file_times(
            &reference_dir_path,
            filetime::FileTime::from_unix_time(1337, 0),
            filetime::FileTime::zero(),
        )
        .unwrap();

        // target_dir should have today's date since it was just created, but reference_dir should be different
        assert_ne!(
            reference_dir_path.metadata().unwrap().accessed().unwrap(),
            target_dir_path.metadata().unwrap().accessed().unwrap()
        );
        assert_ne!(
            reference_dir_path.metadata().unwrap().modified().unwrap(),
            target_dir_path.metadata().unwrap().modified().unwrap()
        );

        nu!(
            cwd: dirs.test(),
            "touch -r reference_dir target_dir"
        );

        assert_eq!(
            reference_dir_path.metadata().unwrap().accessed().unwrap(),
            target_dir_path.metadata().unwrap().accessed().unwrap()
        );
        assert_eq!(
            reference_dir_path.metadata().unwrap().modified().unwrap(),
            target_dir_path.metadata().unwrap().modified().unwrap()
        );
    })
}
