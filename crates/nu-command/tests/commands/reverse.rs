use nu_test_support::prelude::*;

#[test]
fn can_get_reverse_first() -> Result {
    let code = "
        ls
        | sort-by name
        | where type == file
        | reverse
        | first
        | get name
        | path basename
        | str trim
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("utf16.ini")
}

#[test]
fn fail_on_non_iterator() -> Result {
    test()
        .run("1 | reverse")
        .expect_error_code_eq("nu::parser::input_type_mismatch")
}
