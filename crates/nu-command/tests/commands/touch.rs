use chrono::{Date, DateTime, Local, TimeZone};
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
fn change_modified_time_of_file() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -t 201908241230.30 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn create_and_change_modified_time_of_file() {
    Playground::setup("change_time_test_1", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch -t 201908241230 i_will_be_created.txt"
        );

        let path = dirs.test().join("i_will_be_created.txt");
        assert!(path.exists());
        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 0);

        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_modified_time_of_file_no_year() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -t 08241230.12 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2022, 8, 24).and_hms(12, 30, 12);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_modified_time_of_file_no_year_no_second() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -t 08241230 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2022, 8, 24).and_hms(12, 30, 0);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_modified_time_of_files() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("file2.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            "touch -t 1908241230.30 file.txt file2.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn errors_if_change_modified_time_of_file_with_invalid_timestamp() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        let mut outcome = nu!(
            cwd: dirs.test(),
            "touch -t 1908241230.3030 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -t 1908241230.3O file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -t 08241230.3 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -t 8241230 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -t 01908241230 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));
    })
}

#[test]
fn change_modified_time_of_file_to_today() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -m file.txt"
        );

        let path = dirs.test().join("file.txt");

        // Check only the date since the time may not match exactly
        let time: Date<Local> = Local::now().date();
        let actual_time: Date<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap()).date();

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_modified_time_timestamp_precedence() {
    Playground::setup("change_time_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -m -t 201908241230.30 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);

        nu!(
            cwd: dirs.test(),
            "touch -t 201908241230.30 -m file.txt"
        );

        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_modified_time_to_date() {
    Playground::setup("change_time_test_11", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            r#"touch -m -d "August 24, 2019; 12:30:30" file.txt"#
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}
