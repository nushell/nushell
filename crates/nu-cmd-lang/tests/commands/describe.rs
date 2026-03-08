use nu_test_support::nu;

#[test]
fn test_oneof_flattening_in_describe() {
    // Reverse order (already flat)
    let result =
        nu!("[ ...(0..4 | each { {content:[{content:[any]}]} }), {content:[any]} ] | describe");
    assert_eq!(
        result.out,
        "table<content: list<oneof<string, record<content: list<string>>>>>"
    );
}

#[test]
fn test_oneof_flattening_in_describe_reverse_order() {
    // Original problematic order (should now be flat, even if not fully deduped)
    let result =
        nu!("[ {content:[any]} ...(0..4 | each { {content:[{content:[any]}]} }) ] | describe");
    assert!(
        result
            .out
            .contains("table<content: list<oneof<string, record<content: list<string>>")
    );
    assert!(!result.out.contains("oneof<oneof<"));
}

#[test]
fn test_oneof_flattening_in_describe_glob_string() {
    let result =
        nu!("do { let g: glob = \"*.rs\"; [ {content: $g} {content: \"README.rs\"} ] | describe }");
    assert!(result.out.contains("table<content: oneof<glob, string>>"));
    assert!(!result.out.contains("oneof<oneof<"));
}

#[test]
fn test_oneof_flattening_in_describe_glob_string_reverse_order() {
    let result =
        nu!("do { let g: glob = \"*.rs\"; [ {content: \"README.rs\"} {content: $g} ] | describe }");
    assert!(result.out.contains("table<content: oneof<string, glob>>"));
    assert!(!result.out.contains("oneof<oneof<"));
}
