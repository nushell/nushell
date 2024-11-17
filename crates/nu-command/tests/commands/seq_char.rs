use nu_test_support::nu;

#[test]
fn fails_when_first_arg_is_multiple_chars() {
    let actual = nu!("seq char aa z");

    assert!(actual
        .err
        .contains("input should be a single ASCII character"));
}

#[test]
fn fails_when_second_arg_is_multiple_chars() {
    let actual = nu!("seq char a zz");

    assert!(actual
        .err
        .contains("input should be a single ASCII character"));
}

#[test]
fn generates_sequence_from_a_to_e() {
    let actual = nu!("seq char a e | str join ''");

    assert_eq!(actual.out, "abcde");
}

#[test]
fn generates_sequence_from_e_to_a() {
    let actual = nu!("seq char e a | str join ''");

    assert_eq!(actual.out, "edcba");
}

#[test]
fn fails_when_non_ascii_character_is_used_in_first_arg() {
    let actual = nu!("seq char ñ z");

    assert!(actual
        .err
        .contains("input should be a single ASCII character"));
}

#[test]
fn fails_when_non_ascii_character_is_used_in_second_arg() {
    let actual = nu!("seq char a ñ");

    assert!(actual
        .err
        .contains("input should be a single ASCII character"));
}

#[test]
fn joins_sequence_with_pipe() {
    let actual = nu!("seq char a e | str join '|'");

    assert_eq!(actual.out, "a|b|c|d|e");
}
