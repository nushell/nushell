use chrono::{DateTime, Datelike, Duration, Local, TimeZone};
use nu_command::util::parse_relative_time;
use nu_command::{leap_year, parse_months, parse_year};
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
            r#"touch -c file.txt"#
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
fn parse_number_and_year() {
    let date = Local::now();
    let leap_next_year = leap_year(date.year() + 1);
    if date.month() == 2 && date.day() == 29 || leap_next_year && date.month() > 2 {
        assert_eq!(parse_relative_time("1 year"), Some(Duration::days(366)));
    } else {
        assert_eq!(parse_relative_time("1 year"), Some(Duration::days(365)));
    }
}

#[test]
fn parse_number_and_week() {
    assert_eq!(parse_relative_time("2 weeks"), Some(Duration::weeks(2)));
}

#[test]
fn parse_just_week() {
    assert_eq!(parse_relative_time("week"), Some(Duration::weeks(1)));
}

#[test]
fn parse_number_and_day() {
    assert_eq!(parse_relative_time("3 days"), Some(Duration::days(3)));
}

#[test]
fn parse_number_and_day1() {
    assert_eq!(parse_relative_time("-3 days"), Some(Duration::days(-3)));
}

#[test]
fn parse_just_day() {
    assert_eq!(parse_relative_time("day"), Some(Duration::days(1)));
}

#[test]
fn parse_now() {
    assert_eq!(parse_relative_time("now"), Some(Duration::nanoseconds(0)));
}

#[test]
fn parse_yesterday() {
    assert_eq!(parse_relative_time("yesterday"), Some(Duration::days(-1)));
}

#[test]
fn parse_tomorrow() {
    assert_eq!(parse_relative_time("tomorrow"), Some(Duration::days(1)));
}

#[test]
fn parse_invalid() {
    assert_eq!(parse_relative_time("foobar"), None);
}

#[test]
fn parse_number_and_hour() {
    assert_eq!(parse_relative_time("4 hours"), Some(Duration::hours(4)));
}

#[test]
fn parse_just_hour() {
    assert_eq!(parse_relative_time("hour"), Some(Duration::hours(1)));
}

#[test]
fn parse_number_and_minute() {
    assert_eq!(
        parse_relative_time("30 minutes"),
        Some(Duration::minutes(30))
    );
}

#[test]
fn parse_number_and_minute1() {
    assert_eq!(
        parse_relative_time("-30 minutes"),
        Some(Duration::minutes(-30))
    );
}

#[test]
fn parse_just_minute() {
    assert_eq!(parse_relative_time("minute"), Some(Duration::minutes(1)));
}

#[test]
fn parse_number_and_seconds() {
    assert_eq!(
        parse_relative_time("45 seconds"),
        Some(Duration::seconds(45))
    );
}

#[test]
fn parse_just_seconds() {
    assert_eq!(parse_relative_time("second"), Some(Duration::seconds(1)));
}

#[test]
fn parse_relative_time_years_spring_forward() {
    let date = Local.with_ymd_and_hms(2020, 3, 8, 1, 30, 0).unwrap();
    assert_eq!(parse_year(date, 1), (Duration::days(365)));
}

#[test]
fn parse_relative_time_years_leap() {
    let date = Local.with_ymd_and_hms(2020, 2, 8, 1, 30, 0).unwrap();
    assert_eq!(parse_year(date, 1), (Duration::days(366)));
}

#[test]
fn parse_relative_time_years_leap1() {
    let date = Local.with_ymd_and_hms(2019, 12, 8, 1, 30, 0).unwrap();
    assert_eq!(parse_year(date, 1), (Duration::days(366)));
}

#[test]
fn parse_months_from_feb_29_2020() {
    let today = Local.with_ymd_and_hms(2020, 2, 29, 0, 0, 0).unwrap(); // Feb 29, 2020
    assert_eq!(parse_months(today, 1), (Duration::days(29)));
    assert_eq!(parse_months(today, 2), (Duration::days(31 + 29)));
    assert_eq!(parse_months(today, 3), (Duration::days(31 + 30 + 29)));
}

#[test]
fn parse_months_from_jun_30_2020() {
    let today = Local.with_ymd_and_hms(2020, 6, 30, 0, 0, 0).unwrap(); // June 30, 2020
    assert_eq!(parse_months(today, 1), (Duration::days(30)));
    assert_eq!(parse_months(today, 2), (Duration::days(31 + 30)));
}

#[test]
fn test_parse_years() {
    let date = Local.with_ymd_and_hms(2020, 01, 03, 1, 0, 0).unwrap();
    assert_eq!(parse_year(date, 1).num_days(), 366);

    let date = Local.with_ymd_and_hms(2019, 01, 03, 1, 0, 0).unwrap();
    assert_eq!(parse_year(date, 6).num_days(), 2192);

    let date = Local.with_ymd_and_hms(2020, 03, 03, 1, 0, 0).unwrap();
    assert_eq!(parse_year(date, 1).num_days(), 365);

    let date = Local.with_ymd_and_hms(2020, 03, 03, 1, 0, 0).unwrap();
    assert_eq!(parse_year(date, -1).num_days(), -366);

    let date = Local.with_ymd_and_hms(2020, 03, 03, 1, 0, 0).unwrap();
    assert_eq!(parse_year(date, 4).num_days(), 1461);

    let date = Local.with_ymd_and_hms(2020, 02, 03, 1, 0, 0).unwrap();
    assert_eq!(parse_year(date, 4).num_days(), 1462);

    let date = Local.with_ymd_and_hms(2020, 02, 29, 1, 0, 0).unwrap();
    assert_eq!((parse_year(date, -4).num_days()), -1462);
}
#[test]
fn test_parse_months() {
    let date = Local.with_ymd_and_hms(2020, 01, 1, 1, 0, 0).unwrap();
    assert_eq!(parse_months(date, 49).num_days(), 1492);

    let date = Local.with_ymd_and_hms(2024, 01, 1, 1, 0, 0).unwrap();
    assert_eq!(parse_months(date, -49).num_days(), -1492);
}
