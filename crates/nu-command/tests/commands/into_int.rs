use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};
use nu_test_support::{nu, pipeline};

#[test]
fn into_int_filesize() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1kb | into int | each { |it| $it / 1000 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_filesize2() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1kib | into int | each { |it| $it / 1024 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_int() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1024 | into int | each { |it| $it / 1024 }
        "#
    ));

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_binary() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 0x[01010101] | into int
        "#
    ));

    assert!(actual.out.contains("16843009"));
}

#[test]
fn into_int_datetime1() {
    let dt = DateTime::parse_from_rfc3339("1983-04-13T12:09:14.123456789+00:00");
    eprintln!("dt debug {:?}", dt);
    assert_eq!(
        dt,
        Ok(FixedOffset::east_opt(0)
            .unwrap()
            .from_local_datetime(
                &NaiveDate::from_ymd_opt(1983, 4, 13)
                    .unwrap()
                    .and_hms_nano_opt(12, 9, 14, 123456789)
                    .unwrap()
            )
            .unwrap())
    );

    let dt_nano = dt.expect("foo").timestamp_nanos();
    assert_eq!(dt_nano % 1_000_000_000, 123456789);
}

#[test]
fn into_int_datetime2() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        "1983-04-13T12:09:14.123456789-05:00" 
        | into datetime --format "%+" 
        | into int
        "#
    ));

    assert_eq!("419101754123456789", actual.out);
}
