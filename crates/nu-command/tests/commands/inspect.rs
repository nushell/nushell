use nu_test_support::nu;

#[test]
fn inspect_with_empty_pipeline() {
    let actual = nu!("inspect");
    assert!(actual.err.contains("no input value was piped in"));
}

#[test]
fn inspect_with_null() {
    let actual = nu!("null | inspect");
    assert!(actual.err.contains("no input value was piped in"));
}

#[test]
fn inspect_with_empty_list() {
    let actual = nu!("[] | inspect");
    assert!(actual.err.is_empty());
}

#[test]
fn inspect_with_empty_table() {
    let actual = nu!("{} | inspect");
    assert!(actual.err.is_empty());
}
