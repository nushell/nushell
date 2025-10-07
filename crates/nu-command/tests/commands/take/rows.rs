use nu_test_support::nu;

#[test]
fn rows() {
    let sample = r#"
        [[name,   lucky_code];
         [Andrés, 1],
         [JT    , 1],
         [Jason , 2],
         [Yehuda, 1]]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | take 3
            | get lucky_code
            | math sum
        "#
    ));

    assert_eq!(actual.out, "4");
}

#[test]
fn rows_with_no_arguments_should_lead_to_error() {
    let actual = nu!("[1 2 3] | take");

    assert!(actual.err.contains("missing_positional"));
}

#[test]
fn fails_on_string() {
    let actual = nu!(r#""foo bar" | take 2"#);

    assert!(actual.err.contains("command doesn't support"));
}

#[test]
fn takes_bytes() {
    let actual = nu!("(0x[aa bb cc] | take 2) == 0x[aa bb]");

    assert_eq!(actual.out, "true");
}

#[test]
fn takes_bytes_from_stream() {
    let actual = nu!("(1.. | each { 0x[aa bb cc] } | bytes collect | take 2) == 0x[aa bb]");

    assert_eq!(actual.out, "true");
}

#[test]
// covers a situation where `take` used to behave strangely on list<binary> input
fn works_with_binary_list() {
    let actual = nu!(r#"
            ([0x[01 11]] | take 1 | get 0) == 0x[01 11]
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn takes_bytes_and_drops_content_type() {
    let actual = nu!(format!(
        "open {} | take 3 | metadata | get content_type? | describe",
        file!(),
    ));

    assert_eq!(actual.out, "nothing");
}
