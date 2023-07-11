use nu_test_support::{nu, pipeline};
use pretty_assertions::assert_eq;

#[test]
fn const_bool() {
    let inp = &[r#"const x = false"#, r#"$x"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "false");
}

#[test]
fn const_int() {
    let inp = &[r#"const x = 10"#, r#"$x"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "10");
}

#[test]
fn const_float() {
    let inp = &[r#"const x = 1.234"#, r#"$x"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "1.234");
}

#[test]
fn const_binary() {
    let inp = &[r#"const x = 0x[12]"#, r#"$x"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert!(actual.out.contains("12"));
}

#[test]
fn const_datetime() {
    let inp = &[r#"const x = 2021-02-27T13:55:40+00:00"#, r#"$x"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert!(actual.out.contains("Sat, 27 Feb 2021 13:55:40"));
}

#[test]
fn const_list() {
    let inp = &[r#"const x = [ a b c ]"#, r#"$x | describe"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "list<string>");
}

#[test]
fn const_record() {
    let inp = &[r#"const x = { a: 10, b: 20, c: 30 }"#, r#"$x | describe"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "record<a: int, b: int, c: int>");
}

#[test]
fn const_table() {
    let inp = &[
        r#"const x = [[a b c]; [10 20 30] [100 200 300]]"#,
        r#"$x | describe"#,
    ];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "table<a: int, b: int, c: int>");
}

#[test]
fn const_string() {
    let inp = &[r#"const x = "abc""#, r#"$x"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "abc");
}

#[test]
fn const_nothing() {
    let inp = &[r#"const x = $nothing"#, r#"$x | describe"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert_eq!(actual.out, "nothing");
}

#[test]
fn const_unsupported() {
    let inp = &[r#"const x = ('abc' | str length)"#];

    let actual = nu!(cwd: "tests/const_", pipeline(&inp.join("; ")));

    assert!(actual.err.contains("not_a_constant"));
}

#[test]
fn const_with_no_spaces_1() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x=4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn const_with_no_spaces_2() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x =4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn const_with_no_spaces_3() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x= 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn const_with_no_spaces_4() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x: int= 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn const_with_no_spaces_5() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x:int= 4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn const_with_no_spaces_6() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x:int=4; $x")
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn const_with_no_spaces_7() {
    let actual = nu!(
        cwd: ".",
        pipeline("const x : int = 4; $x")
    );

    assert_eq!(actual.out, "4");
}
