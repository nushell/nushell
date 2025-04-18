use nu_test_support::nu;

// Tests happy paths

#[test]
fn into_duration_from_record() {
    let actual = nu!(
        r#"{week: 10, day: 1, hour: 2, minute: 3, second: 4, millisecond: 5, microsecond: 6, nanosecond: 7, sign: '+'} | into duration | into record"#
    );
    let expected = nu!(
        r#"{week: 10, day: 1, hour: 2, minute: 3, second: 4, millisecond: 5, microsecond: 6, nanosecond: 7, sign: '+'}"#
    );

    assert_eq!(expected.out, actual.out);
}

#[test]
fn into_duration_from_record_negative() {
    let actual = nu!(
        r#"{week: 10, day: 1, hour: 2, minute: 3, second: 4, millisecond: 5, microsecond: 6, nanosecond: 7, sign: '-'} | into duration | into record"#
    );
    let expected = nu!(
        r#"{week: 10, day: 1, hour: 2, minute: 3, second: 4, millisecond: 5, microsecond: 6, nanosecond: 7, sign: '-'}"#
    );

    assert_eq!(expected.out, actual.out);
}

#[test]
fn into_duration_from_record_defaults() {
    let actual = nu!(r#"{} | into duration | into int"#);

    assert_eq!("0".to_string(), actual.out);
}

#[test]
fn into_duration_from_record_round_trip() {
    let actual = nu!(
        r#"('10wk 1day 2hr 3min 4sec 5ms 6µs 7ns' | into duration | into record | into duration | into string) == '10wk 1day 2hr 3min 4sec 5ms 6µs 7ns'"#
    );

    assert!(actual.out.contains("true"));
}

#[test]
fn into_duration_table_column() {
    let actual =
        nu!(r#"[[value]; ['1sec'] ['2min'] ['3hr'] ['4day'] ['5wk']] | into duration value"#);
    let expected = nu!(r#"[[value]; [1sec] [2min] [3hr] [4day] [5wk]]"#);

    assert_eq!(actual.out, expected.out);
}
