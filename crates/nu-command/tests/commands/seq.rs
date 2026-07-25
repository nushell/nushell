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

#[test]
fn zero_increment_is_rejected() {
    // Previously `seq 5 0 5` looped forever emitting `5`; a zero increment
    // must error instead (matching GNU seq).
    let actual = nu!("seq 5 0 5");

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("increment cannot be 0"));
}

#[test]
fn zero_increment_is_rejected_for_empty_range() {
    // `seq 1 0 5` previously produced an empty list silently; it must error too.
    let actual = nu!("seq 1 0 5");

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("increment cannot be 0"));
}

#[test]
fn zero_float_increment_is_rejected() {
    let actual = nu!("seq 1.0 0.0 5.0");

    assert!(actual.out.is_empty());
    assert!(actual.err.contains("increment cannot be 0"));
}

#[test]
fn int_sequence_at_max_does_not_panic() {
    // Advancing past i64::MAX previously panicked with "attempt to add with
    // overflow"; the final in-range value must still be emitted and the
    // sequence must end cleanly.
    let actual = nu!("seq 9223372036854775807 9223372036854775807 | to nuon");

    assert_eq!(actual.out, "[9223372036854775807]");
    assert!(!actual.err.contains("overflow"));
}
