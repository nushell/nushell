use nu_test_support::prelude::*;

#[test]
fn test_char_list_outputs_table() -> Result {
    test().run("char --list | length").expect_value_eq(113)
}

#[test]
fn test_char_eol() -> Result {
    let code = r#"
        let expected = if ($nu.os-info.name == 'windows') { "\r\n" } else { "\n" }
        ((char lsep) == $expected) and ((char line_sep) == $expected) and ((char eol) == $expected)
    "#;

    test().run(code).expect_value_eq(true)
}
