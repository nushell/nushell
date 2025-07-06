use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone};
use rstest::rstest;

use nu_test_support::nu;

#[test]
fn into_int_filesize() {
    let actual = nu!("echo 1kb | into int | each { |it| $it / 1000 }");

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_filesize2() {
    let actual = nu!("echo 1kib | into int | each { |it| $it / 1024 }");

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_int() {
    let actual = nu!("echo 1024 | into int | each { |it| $it / 1024 }");

    assert!(actual.out.contains('1'));
}

#[test]
fn into_int_binary() {
    let actual = nu!("echo 0x[01010101] | into int");

    assert!(actual.out.contains("16843009"));
}

#[test]
fn into_int_binary_signed() {
    let actual = nu!("echo 0x[f0] | into int --signed");

    assert!(actual.out.contains("-16"));
}

#[test]
fn into_int_binary_back_and_forth() {
    let actual = nu!("echo 0x[f0] | into int | into binary | to nuon");

    assert!(actual.out.contains("0x[F000000000000000]"));
}

#[test]
fn into_int_binary_signed_back_and_forth() {
    let actual = nu!("echo 0x[f0] | into int --signed | into binary | to nuon");

    assert!(actual.out.contains("0x[F0FFFFFFFFFFFFFF]"));
}

#[test]
fn into_int_binary_empty() {
    let actual = nu!("echo 0x[] | into int");

    assert!(actual.out.contains('0'))
}

#[test]
fn into_int_binary_signed_empty() {
    let actual = nu!("echo 0x[] | into int --signed");

    assert!(actual.out.contains('0'))
}

#[test]
#[ignore]
fn into_int_datetime1() {
    let dt = DateTime::parse_from_rfc3339("1983-04-13T12:09:14.123456789+00:00");
    eprintln!("dt debug {dt:?}");
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

    let dt_nano = dt.expect("foo").timestamp_nanos_opt().unwrap_or_default();
    assert_eq!(dt_nano % 1_000_000_000, 123456789);
}

#[rstest]
#[case("1983-04-13T12:09:14.123456789-05:00", "419101754123456789")] // full precision
#[case("1983-04-13T12:09:14.456789-05:00", "419101754456789000")] // microsec
#[case("1983-04-13T12:09:14-05:00", "419101754000000000")] // sec
#[case("2052-04-13T12:09:14.123456789-05:00", "2596640954123456789")] // future date > 2038 epoch
#[case("1902-04-13T12:09:14.123456789-05:00", "-2137042245876543211")] // past date < 1970
fn into_int_datetime(#[case] time_in: &str, #[case] int_out: &str) {
    let actual = nu!(&format!(
        r#""{time_in}" | into datetime --format "%+" | into int"#
    ));

    assert_eq!(int_out, actual.out);
}
