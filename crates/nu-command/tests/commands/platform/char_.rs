use nu_test_support::nu;

#[test]
fn test_char_list_outputs_table() {
    let actual = nu!(r#"
        char --list | length
    "#);

    assert_eq!(actual.out, "113");
}

#[test]
fn test_char_eol() {
    let actual = nu!(r#"
        let expected = if ($nu.os-info.name == 'windows') { "\r\n" } else { "\n" }
        ((char lsep) == $expected) and ((char line_sep) == $expected) and ((char eol) == $expected)
    "#);

    assert_eq!(actual.out, "true");
}
