use nu_test_support::nu;

#[test]
fn float_in_seq_leads_to_lists_of_floats() {
    let actual = nu!("seq 1.0 0.5 6 | describe");

    assert_eq!(actual.out, "list<float> (stream)");
}

#[test]
fn ints_in_seq_leads_to_lists_of_ints() {
    let actual = nu!("seq 1 2 6 | describe");

    assert_eq!(actual.out, "list<int> (stream)");
}
