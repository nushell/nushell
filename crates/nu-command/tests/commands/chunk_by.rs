use nu_test_support::nu;

#[test]
fn chunk_by_on_empty_input_returns_empty_list() {
    let actual = nu!("[] | chunk-by {|it| $it} | to nuon");
    assert!(actual.err.is_empty());
    assert_eq!(actual.out, "[]");
}

#[test]
fn chunk_by_strings_works() {
    let sample = r#"[a a a b b b c c c a a a]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | chunk-by {{|it| $it}}
            | to nuon
        "#
    ));

    assert_eq!(actual.out, "[[a, a, a], [b, b, b], [c, c, c], [a, a, a]]");
}

#[test]
fn chunk_by_field_works() {
    let sample = r#"[
    {
        name: bob,
        age: 20,
        cool: false
    },
    {
        name: jane,
        age: 30,
        cool: false
    },
    {
        name: marie,
        age: 19,
        cool: true
    },
    {
        name: carl,
        age: 36,
        cool: true
    } ]"#;

    let actual = nu!(format!(
        r#"
            {sample}
            | chunk-by {{|it| $it.cool}}
            | length
        "#
    ));

    assert_eq!(actual.out, "2");
}
