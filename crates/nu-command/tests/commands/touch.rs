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
    Playground::setup("change_time_test_3", |dirs, sandbox| {
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
    })
}

#[test]
fn create_and_change_modified_time_of_file() {
    Playground::setup("change_time_test_4", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch -m -t 201908241230 i_will_be_created.txt"
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
    Playground::setup("change_time_test_5", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -m -t 08241230.12 file.txt"
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
    Playground::setup("change_time_test_6", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -m -t 08241230 file.txt"
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
    Playground::setup("change_time_test_7", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("file2.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            "touch -m -t 1908241230.30 file.txt file2.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);

        let path = dirs.test().join("file2.txt");

        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn errors_if_change_modified_time_of_file_with_invalid_timestamp() {
    Playground::setup("change_time_test_8", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        let mut outcome = nu!(
            cwd: dirs.test(),
            "touch -m -t 1908241230.3030 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -m -t 1908241230.3O file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -m -t 08241230.3 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -m -t 8241230 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -m -t 01908241230 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));
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
        let date: Date<Local> = Local::now().date();
        let actual_date: Date<Local> =
            DateTime::from(path.metadata().unwrap().modified().unwrap()).date();

        assert_eq!(date, actual_date);
    })
}

#[test]
fn change_modified_time_to_date() {
    Playground::setup("change_time_test_10", |dirs, sandbox| {
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

#[test]
fn change_modified_time_to_time_of_reference() {
    Playground::setup("change_time_test_11", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("reference.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            r#"touch -m -t 201908241230.30 reference.txt"#
        );

        nu!(
            cwd: dirs.test(),
            r#"touch -m -r reference.txt file.txt"#
        );

        let path = dirs.test().join("file.txt");
        let ref_path = dirs.test().join("reference.txt");

        let time: DateTime<Local> = DateTime::from(path.metadata().unwrap().modified().unwrap());
        let ref_time: DateTime<Local> =
            DateTime::from(ref_path.metadata().unwrap().modified().unwrap());

        assert_eq!(time, ref_time);
    })
}

#[test]
fn change_access_time_of_file() {
    Playground::setup("change_time_test_12", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a -t 201908241230.30 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn create_and_change_access_time_of_file() {
    Playground::setup("change_time_test_13", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch -a -t 201908241230 i_will_be_created.txt"
        );

        let path = dirs.test().join("i_will_be_created.txt");
        assert!(path.exists());
        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 0);

        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_access_time_of_file_no_year() {
    Playground::setup("change_time_test_14", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a -t 08241230.12 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2022, 8, 24).and_hms(12, 30, 12);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_access_time_of_file_no_year_no_second() {
    Playground::setup("change_time_test_15", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a -t 08241230 file.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2022, 8, 24).and_hms(12, 30, 0);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_access_time_of_files() {
    Playground::setup("change_time_test_16", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("file2.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            "touch -a -t 1908241230.30 file.txt file2.txt"
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);

        let path = dirs.test().join("file2.txt");

        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn errors_if_change_access_time_of_file_with_invalid_timestamp() {
    Playground::setup("change_time_test_17", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        let mut outcome = nu!(
            cwd: dirs.test(),
            "touch -a -t 1908241230.3030 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -a -t 1908241230.3O file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -a -t 08241230.3 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -a -t 8241230 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -a -t 01908241230 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));
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
        let date: Date<Local> = Local::now().date();
        let actual_date: Date<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap()).date();

        assert_eq!(date, actual_date);
    })
}

#[test]
fn change_access_time_to_date() {
    Playground::setup("change_time_test_19", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            r#"touch -a -d "August 24, 2019; 12:30:30" file.txt"#
        );

        let path = dirs.test().join("file.txt");

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let actual_time: DateTime<Local> =
            DateTime::from(path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, actual_time);
    })
}

#[test]
fn change_access_time_to_time_of_reference() {
    Playground::setup("change_time_test_20", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("reference.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            r#"touch -a -t 201908241230.30 reference.txt"#
        );

        nu!(
            cwd: dirs.test(),
            r#"touch -a -r reference.txt file.txt"#
        );

        let path = dirs.test().join("file.txt");
        let ref_path = dirs.test().join("reference.txt");

        let time: DateTime<Local> = DateTime::from(path.metadata().unwrap().accessed().unwrap());
        let ref_time: DateTime<Local> =
            DateTime::from(ref_path.metadata().unwrap().accessed().unwrap());

        assert_eq!(time, ref_time);
    })
}

#[test]
fn change_modified_and_access_time_of_file() {
    Playground::setup("change_time_test_21", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -m -a -t 201908241230.30 file.txt"
        );

        let path = dirs.test().join("file.txt");
        let metadata = path.metadata().unwrap();

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);
    })
}

#[test]
fn create_and_change_modified_and_access_time_of_file() {
    Playground::setup("change_time_test_22", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            "touch -t 201908241230 i_will_be_created.txt"
        );

        let path = dirs.test().join("i_will_be_created.txt");
        assert!(path.exists());

        let metadata = path.metadata().unwrap();

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 0);

        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);
    })
}

#[test]
fn change_modified_and_access_time_of_file_no_year() {
    Playground::setup("change_time_test_23", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a -m -t 08241230.12 file.txt"
        );

        let metadata = dirs.test().join("file.txt").metadata().unwrap();

        let time = Local.ymd(2022, 8, 24).and_hms(12, 30, 12);

        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);
    })
}

#[test]
fn change_modified_and_access_time_of_file_no_year_no_second() {
    Playground::setup("change_time_test_24", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -t 08241230 file.txt"
        );

        let metadata = dirs.test().join("file.txt").metadata().unwrap();

        let time = Local.ymd(2022, 8, 24).and_hms(12, 30, 0);

        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);
    })
}

#[test]
fn change_modified_and_access_time_of_files() {
    Playground::setup("change_time_test_25", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("file2.txt"),
        ]);

        nu!(
            cwd: dirs.test(),
            "touch -a -m -t 1908241230.30 file.txt file2.txt"
        );

        let metadata = dirs.test().join("file.txt").metadata().unwrap();

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);

        let metadata = dirs.test().join("file2.txt").metadata().unwrap();

        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);
    })
}

#[test]
fn errors_if_change_modified_and_access_time_of_file_with_invalid_timestamp() {
    Playground::setup("change_time_test_26", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        let mut outcome = nu!(
            cwd: dirs.test(),
            "touch -t 1908241230.3030 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -a -m -t 1908241230.3O file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -t 08241230.3 file.txt"
        );

        assert!(outcome.err.contains("input has an invalid timestamp"));

        outcome = nu!(
            cwd: dirs.test(),
            "touch -m -a -t 8241230 file.txt"
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
fn change_modified_and_access_time_of_file_to_today() {
    Playground::setup("change_time_test_27", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            "touch -a -m file.txt"
        );

        let metadata = dirs.test().join("file.txt").metadata().unwrap();

        // Check only the date since the time may not match exactly
        let date: Date<Local> = Local::now().date();
        let adate: Date<Local> = DateTime::from(metadata.accessed().unwrap()).date();
        let mdate: Date<Local> = DateTime::from(metadata.modified().unwrap()).date();

        assert_eq!(date, adate);
        assert_eq!(date, mdate);
    })
}

#[test]
fn change_modified_and_access_time_to_date() {
    Playground::setup("change_time_test_28", |dirs, sandbox| {
        sandbox.with_files(vec![Stub::EmptyFile("file.txt")]);

        nu!(
            cwd: dirs.test(),
            r#"touch -d "August 24, 2019; 12:30:30" file.txt"#
        );

        let metadata = dirs.test().join("file.txt").metadata().unwrap();

        let time = Local.ymd(2019, 8, 24).and_hms(12, 30, 30);
        let atime: DateTime<Local> = DateTime::from(metadata.accessed().unwrap());
        let mtime: DateTime<Local> = DateTime::from(metadata.modified().unwrap());

        assert_eq!(time, atime);
        assert_eq!(time, mtime);
    })
}

#[test]
fn change_modified_and_access_time_to_time_of_reference() {
    Playground::setup("change_time_test_29", |dirs, sandbox| {
        sandbox.with_files(vec![
            Stub::EmptyFile("file.txt"),
            Stub::EmptyFile("reference.txt"),
        ]);

        let path = dirs.test().join("file.txt");
        let ref_path = dirs.test().join("reference.txt");

        // Set the same time for the modified and access time of the reference file
        nu!(
            cwd: dirs.test(),
            r#"touch -a -m -t 201908241230.30 reference.txt"#
        );

        nu!(
            cwd: dirs.test(),
            r#"touch -r reference.txt file.txt"#
        );

        let atime: DateTime<Local> = DateTime::from(path.metadata().unwrap().accessed().unwrap());
        let ref_atime: DateTime<Local> =
            DateTime::from(ref_path.metadata().unwrap().accessed().unwrap());

        assert_eq!(atime, ref_atime);

        let mtime: DateTime<Local> = DateTime::from(path.metadata().unwrap().modified().unwrap());
        let ref_mtime: DateTime<Local> =
            DateTime::from(ref_path.metadata().unwrap().modified().unwrap());

        assert_eq!(mtime, ref_mtime);

        // Set different time for the modified and access time of the reference file
        nu!(
            cwd: dirs.test(),
            r#"touch -a -t 201908241230.30 reference.txt"#
        );

        nu!(
            cwd: dirs.test(),
            r#"touch -m -t 202009251340.40 reference.txt"#
        );

        nu!(
            cwd: dirs.test(),
            r#"touch -a -m -r reference.txt file.txt"#
        );

        let atime: DateTime<Local> = DateTime::from(path.metadata().unwrap().accessed().unwrap());
        let ref_atime: DateTime<Local> =
            DateTime::from(ref_path.metadata().unwrap().accessed().unwrap());

        assert_eq!(atime, ref_atime);

        let mtime: DateTime<Local> = DateTime::from(path.metadata().unwrap().modified().unwrap());
        let ref_mtime: DateTime<Local> =
            DateTime::from(ref_path.metadata().unwrap().modified().unwrap());

        assert_eq!(mtime, ref_mtime);
    })
}

#[test]
fn not_create_file_if_it_not_exists() {
    Playground::setup("change_time_test_28", |dirs, _sandbox| {
        nu!(
            cwd: dirs.test(),
            r#"touch -c -d "August 24, 2019; 12:30:30" file.txt"#
        );

        let path = dirs.test().join("file.txt");

        assert!(!path.exists());

        nu!(
            cwd: dirs.test(),
            r#"touch -c file.txt"#
        );

        let path = dirs.test().join("file.txt");

        assert!(!path.exists());
    })
}
