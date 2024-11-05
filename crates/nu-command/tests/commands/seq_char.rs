use nu_test_support::nu;

#[test]
fn fails_when_first_arg_is_multiple_chars() {
    let actual = nu!("seq char aa z");

    assert!(actual.err.contains("should be 1 character long"));
}

#[test]
fn fails_when_second_arg_is_multiple_chars() {
    let actual = nu!("seq char a zz");

    assert!(actual.err.contains("should be 1 character long"));
}

#[test]
fn generates_ascii_sequence_correctly() {
    let actual = nu!("seq char a e");
    
    assert!(actual.out == "a\nb\nc\nd\ne");
}

#[test]
fn generates_ascii_sequence_with_graphic_flag() {
    let actual = nu!("seq char '!' '/' --graphic");

    // Expected output is only graphic characters between '!' and '/'
    let expected_output = "!\n\"\n#\n$\n%\n&\n'\n(\n)\n*\n+\n,\n-\n.\n/";
    assert!(actual.out == expected_output);
}

#[test]
fn excludes_non_graphic_characters_with_graphic_flag() {
    let actual = nu!("seq char '\x1F' 'B' --graphic");

    // Expected output is only graphic characters from the ASCII range, excluding control characters
    let expected_output = "!\n\"\n#\n$\n%\n&\n'\n(\n)\n*\n+\n,\n-\n.\n/\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n:\n;\n<\n=\n>\n?\n@\nA\nB";
    assert!(actual.out == expected_output);
}

#[test]
fn fails_when_input_is_not_ascii() {
    let actual = nu!("seq char 'ñ' 'z'");

    assert!(actual.err.contains("seq char only accepts individual ASCII characters as parameters"));
}

#[test]
fn accepts_full_ascii_range_without_flag() {
    let actual = nu!("seq char '!' 'C'");

    // Expected output includes all ASCII characters from '!' to 'C'
    let expected_output = "!\n\"\n#\n$\n%\n&\n'\n(\n)\n*\n+\n,\n-\n.\n/\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n:\n;\n<\n=\n>\n?\n@\nA\nB\nC";
    assert!(actual.out == expected_output);
}