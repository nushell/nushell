use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case("char --list | length", 56)]
#[case("char --list | get aliases | flatten | length", 62)]
fn test_char_list_outputs_table(#[case] code: &str, #[case] expect: u32) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[test]
fn test_char_eol() -> Result {
    let code = r#"
        let expected = if ($nu.os-info.name == 'windows') { "\r\n" } else { "\n" }
        ((char lsep) == $expected) and ((char line_sep) == $expected) and ((char eol) == $expected)
    "#;

    test().run(code).expect_value_eq(true)
}
