use nu_test_support::nu;

#[test]
fn test_oneof_flattening_in_describe() {
    // Test case 1: Reverse order (already flat)
    let result1 =
        nu!("[ ...(0..4 | each { {content:[{content:[any]}]} }), {content:[any]} ] | describe");
    assert_eq!(
        result1.out,
        "table<content: list<oneof<string, record<content: list<string>>>>>"
    );

    // Test case 2: Original problematic order (should now be flat, even if not fully deduped)
    let result2 =
        nu!("[ {content:[any]} ...(0..4 | each { {content:[{content:[any]}]} }) ] | describe");
    assert!(
        result2
            .out
            .contains("table<content: list<oneof<string, record<content: list<string>>")
    );
    assert!(!result2.out.contains("oneof<oneof<"));
}
