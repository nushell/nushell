use nu_test_support::nu;
use pretty_assertions::assert_eq;

#[test]
fn const_bool() {
    let inp = &["const x = false", "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "false");
}

#[test]
fn const_int() {
    let inp = &["const x = 10", "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "10");
}

#[test]
fn const_float() {
    let inp = &["const x = 1.234", "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "1.234");
}

#[test]
fn const_binary() {
    let inp = &["const x = 0x[12]", "$x"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.out.contains("12"));
}

#[test]
fn const_datetime() {
    let inp = &["const x = 2021-02-27T13:55:40+00:00", "$x"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.out.contains("Sat, 27 Feb 2021 13:55:40"));
}

#[test]
fn const_list() {
    let inp = &["const x = [ a b c ]", "$x | describe"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "list<string>");
}

#[test]
fn const_record() {
    let inp = &["const x = { a: 10, b: 20, c: 30 }", "$x | describe"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "record<a: int, b: int, c: int>");
}

#[test]
fn const_table() {
    let inp = &[
        "const x = [[a b c]; [10 20 30] [100 200 300]]",
        "$x | describe",
    ];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "table<a: int, b: int, c: int>");
}

#[test]
fn const_string() {
    let inp = &[r#"const x = "abc""#, "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "abc");
}

#[test]
fn const_nothing() {
    let inp = &["const x = $nothing", "$x | describe"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "nothing");
}

#[test]
fn const_subexpression_supported() {
    let inp = &["const x = ('spam')", "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "spam");
}

#[test]
fn const_command_supported() {
    let inp = &["const x = ('spam' | str length)", "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "4");
}

#[test]
fn const_command_unsupported() {
    let inp = &["const x = (loop { break })"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("not_a_const_command"));
}

#[test]
fn const_in_scope() {
    let inp = &["do { const x = 'x'; $x }"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "x");
}

#[test]
fn not_a_const_help() {
    let actual = nu!("const x = ('abc' | str length -h)");
    assert!(actual.err.contains("not_a_const_help"));
}

// const implementations of commands without dedicated tests
#[test]
fn describe_const() {
    let actual = nu!("const x = ('abc' | describe); $x");
    assert_eq!(actual.out, "string");
}

#[test]
fn ignore_const() {
    let actual = nu!("const x = (echo spam | ignore); $x == null");
    assert_eq!(actual.out, "true");
}

#[test]
fn version_const() {
    let actual = nu!("const x = (version); $x");
    assert!(actual.err.is_empty());
}
