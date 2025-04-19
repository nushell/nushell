use nu_test_support::nu;

// Tests happy paths

#[test]
fn into_duration_float() {
    let actual = nu!(r#"1.07min | into duration"#);

    assert_eq!("1min 4sec 200ms", actual.out);
}
