use nu_test_support::nu;
use nu_test_support::prelude::*;

#[test]
fn test_oneof_flattening_in_describe() -> Result {
    // Reverse order (already flat)

    test()
        .run("[ ...(0..4 | each { {content:[{content:[any]}]} }), {content:[any]} ] | describe")
        .expect_value_eq("table<content: oneof<table<content: list<string>>, list<string>>>")
}

#[test]
fn test_oneof_flattening_in_describe_reverse_order() -> Result {
    // Original problematic order `oneof<oneof<` (should now be flat, even if not fully deduped)
    test()
        .run("[ {content:[any]} ...(0..4 | each { {content:[{content:[any]}]} }) ] | describe")
        .expect_value_eq("table<content: oneof<list<string>, table<content: list<string>>>>")
}

#[test]
fn test_oneof_flattening_in_describe_glob_string() -> Result {
    test()
        .run("do { let g: glob = \"*.rs\"; [ {content: $g} {content: \"README.rs\"} ] | describe }")
        .expect_value_eq("table<content: oneof<glob, string>>")
}

#[test]
fn test_oneof_flattening_in_describe_glob_string_reverse_order() -> Result {
    test()
        .run("do { let g: glob = \"*.rs\"; [ {content: \"README.rs\"} {content: $g} ] | describe }")
        .expect_value_eq("table<content: oneof<string, glob>>")
}
