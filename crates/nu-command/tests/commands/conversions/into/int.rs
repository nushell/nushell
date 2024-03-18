use nu_test_support::nu;

#[test]
fn convert_back_and_forth() {
    let actual = nu!(r#"1 | into binary | into int"#);
    assert_eq!(actual.out, "1");
}

#[test]
fn convert_into_int_little_endian() {
    let actual = nu!(r#"0x[01 00 00 00 00 00 00 00] | into int --endian little"#);
    assert_eq!(actual.out, "1");

    let actual = nu!(r#"0x[00 00 00 00 00 00 00 01] | into int --endian little"#);
    assert_eq!(actual.out, "72057594037927936");
}

#[test]
fn convert_into_int_big_endian() {
    let actual = nu!(r#"0x[00 00 00 00 00 00 00 01] | into int --endian big"#);
    assert_eq!(actual.out, "1");

    let actual = nu!(r#"0x[01 00 00 00 00 00 00 00] | into int --endian big"#);
    assert_eq!(actual.out, "72057594037927936");
}

#[test]
fn convert_into_signed_int_little_endian() {
    let actual = nu!(r#"0x[ff 00 00 00 00 00 00 00] | into int --endian little --signed"#);
    assert_eq!(actual.out, "255");

    let actual = nu!(r#"0x[00 00 00 00 00 00 00 ff] | into int --endian little --signed"#);
    assert_eq!(actual.out, "-72057594037927936");
}

#[test]
fn convert_into_signed_int_big_endian() {
    let actual = nu!(r#"0x[00 00 00 00 00 00 00 ff] | into int --endian big"#);
    assert_eq!(actual.out, "255");

    let actual = nu!(r#"0x[ff 00 00 00 00 00 00 00] | into int --endian big"#);
    assert_eq!(actual.out, "-72057594037927936");
}
