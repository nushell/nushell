use nu_test_support::{nu, pipeline};

#[test]
fn rows() {
    let sample = r#"
        [[name,   lucky_code];
         [Andrés, 1],
         [JT    , 1],
         [Jason , 2],
         [Yehuda, 1]]"#;

    let actual = nu!(pipeline(&format!(
        r#"
                {}
                | take 3
                | get lucky_code
                | math sum
                "#,
        &sample
    )));

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
// covers a situation where `take` used to behave strangely on list<binary> input
fn works_with_binary_list() {
    let actual = nu!(r#"
            ([0x[01 11]] | take 1 | get 0) == 0x[01 11]
        "#);

    assert_eq!(actual.out, "true");
}
