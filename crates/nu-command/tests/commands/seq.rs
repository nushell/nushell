use nu_test_support::{nu, pipeline};

#[test]
fn float_in_seq_leads_to_lists_of_floats() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        seq 1.0 0.5 6 | describe
        "#
    ));

    assert_eq!(actual.out, "list<float>");
}

#[test]
fn ints_in_seq_leads_to_lists_of_ints() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
        seq 1 2 6 | describe
        "#
    ));

    assert_eq!(actual.out, "list<int>");
}
