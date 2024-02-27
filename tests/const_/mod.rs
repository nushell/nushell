use nu_test_support::nu;
use pretty_assertions::assert_eq;
use rstest::rstest;

const MODULE_SETUP: &str = r#"
    module spam {
        export const X = 'x'
        export module eggs {
            export const E = 'e'
            export module bacon {
                export const viking = 'eats'
                export module none {}
            }
        }
    }
"#;

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
fn const_invalid_table() {
    let inp = &["const x = [[a b a]; [10 20 30] [100 200 300]]"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("column_defined_twice"));
}

#[test]
fn const_string() {
    let inp = &[r#"const x = "abc""#, "$x"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "abc");
}

#[test]
fn const_string_interpolation() {
    let actual = nu!(r#"
        const x = 2
        const s = $"var: ($x), date: (2021-02-27T13:55:40+00:00), file size: (2kb)"
        $s
    "#);
    assert_eq!(
        actual.out,
        "var: 2, date: Sat, 27 Feb 2021 13:55:40 +0000 (3 years ago), file size: 2.0 KiB"
    );
}

#[test]
fn const_nothing() {
    let inp = &["const x = null", "$x | describe"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "nothing");
}

#[rstest]
#[case(&["const x = not false", "$x"], "true")]
#[case(&["const x = false", "const y = not $x", "$y"], "true")]
#[case(&["const x = not false", "const y = not $x", "$y"], "false")]
fn const_unary_operator(#[case] inp: &[&str], #[case] expect: &str) {
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, expect);
}

#[rstest]
#[case(&["const x = 1 + 2", "$x"], "3")]
#[case(&["const x = 1 * 2", "$x"], "2")]
#[case(&["const x = 4 / 2", "$x"], "2")]
#[case(&["const x = 4 mod 3", "$x"], "1")]
#[case(&["const x = 5.0 / 2.0", "$x"], "2.5")]
#[case(&[r#"const x = "a" + "b" "#, "$x"], "ab")]
#[case(&[r#"const x = "a" ++ "b" "#, "$x"], "ab")]
#[case(&[r#"const x = [1,2] ++ [3]"#, "$x | describe"], "list<int>")]
#[case(&[r#"const x = 0x[1,2] ++ 0x[3]"#, "$x | describe"], "binary")]
#[case(&[r#"const x = 0x[1,2] ++ [3]"#, "$x | describe"], "list<any>")]
#[case(&["const x = 1 < 2", "$x"], "true")]
#[case(&["const x = (3 * 200) > (2 * 100)", "$x"], "true")]
#[case(&["const x = (3 * 200) < (2 * 100)", "$x"], "false")]
#[case(&["const x = (3 * 200) == (2 * 300)", "$x"], "true")]
fn const_binary_operator(#[case] inp: &[&str], #[case] expect: &str) {
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, expect);
}

#[rstest]
#[case(&["const x = 1 / 0", "$x"], "division by zero")]
#[case(&["const x = 10 ** 10000000", "$x"], "pow operation overflowed")]
#[case(&["const x = 2 ** 62 * 2", "$x"], "multiply operation overflowed")]
#[case(&["const x = 1 ++ 0", "$x"], "doesn't support this value")]
fn const_operator_error(#[case] inp: &[&str], #[case] expect: &str) {
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains(expect));
}

#[rstest]
#[case(&["const x = (1..3)", "$x | math sum"], "6")]
#[case(&["const x = (1..3)", "$x | describe"], "range")]
fn const_range(#[case] inp: &[&str], #[case] expect: &str) {
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, expect);
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

#[test]
fn complex_const_export() {
    let inp = &[MODULE_SETUP, "use spam", "$spam.X"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "x");

    let inp = &[MODULE_SETUP, "use spam", "$spam.eggs.E"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "e");

    let inp = &[MODULE_SETUP, "use spam", "$spam.eggs.bacon.viking"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "eats");

    let inp = &[
        MODULE_SETUP,
        "use spam",
        "($spam.eggs.bacon.none | is-empty)",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "true");
}

#[test]
fn complex_const_glob_export() {
    let inp = &[MODULE_SETUP, "use spam *", "$X"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "x");

    let inp = &[MODULE_SETUP, "use spam *", "$eggs.E"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "e");

    let inp = &[MODULE_SETUP, "use spam *", "$eggs.bacon.viking"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "eats");

    let inp = &[MODULE_SETUP, "use spam *", "($eggs.bacon.none | is-empty)"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "true");
}

#[test]
fn complex_const_drill_export() {
    let inp = &[
        MODULE_SETUP,
        "use spam eggs bacon none",
        "($none | is-empty)",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "true");
}

#[test]
fn complex_const_list_export() {
    let inp = &[
        MODULE_SETUP,
        "use spam [X eggs]",
        "[$X $eggs.E] | str join ''",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "xe");
}

#[test]
fn exported_const_is_const() {
    let module1 = "module foo {
        export def main [] { 'foo' }
    }";

    let module2 = "module spam {
        export const MOD_NAME = 'foo'
    }";

    let inp = &[module1, module2, "use spam", "use $spam.MOD_NAME", "foo"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "foo");
}

#[test]
fn const_captures_work() {
    let module = "module spam {
        export const X = 'x'
        const Y = 'y'

        export-env { $env.SPAM = $X + $Y }
        export def main [] { $X + $Y }
    }";

    let inp = &[module, "use spam", "$env.SPAM"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "xy");

    let inp = &[module, "use spam", "spam"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "xy");
}

#[test]
fn const_captures_in_closures_work() {
    let module = "module foo {
        const a = 'world'
        export def bar [] {
            'hello ' + $a
        }
    }";
    let inp = &[module, "use foo", "do { foo bar }"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "hello world");
}

#[ignore = "TODO: Need to fix `overlay hide` to hide the constants brough by `overlay use`"]
#[test]
fn complex_const_overlay_use_hide() {
    let inp = &[MODULE_SETUP, "overlay use spam", "$X"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "x");

    let inp = &[MODULE_SETUP, "overlay use spam", "$eggs.E"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "e");

    let inp = &[MODULE_SETUP, "overlay use spam", "$eggs.bacon.viking"];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "eats");

    let inp = &[
        MODULE_SETUP,
        "overlay use spam",
        "($eggs.bacon.none | is-empty)",
    ];
    let actual = nu!(&inp.join("; "));
    assert_eq!(actual.out, "true");

    let inp = &[MODULE_SETUP, "overlay use spam", "overlay hide", "$eggs"];
    let actual = nu!(&inp.join("; "));
    assert!(actual.err.contains("nu::parser::variable_not_found"));
}

// const implementations of commands without dedicated tests
#[test]
fn describe_const() {
    let actual = nu!("const x = ('abc' | describe); $x");
    assert_eq!(actual.out, "string");
}

#[test]
fn ignore_const() {
    let actual = nu!(r#"const x = ("spam" | ignore); $x == null"#);
    assert_eq!(actual.out, "true");
}

#[test]
fn version_const() {
    let actual = nu!("const x = (version); $x");
    assert!(actual.err.is_empty());
}

#[test]
fn if_const() {
    let actual = nu!("const x = (if 2 < 3 { 'yes!' }); $x");
    assert_eq!(actual.out, "yes!");

    let actual = nu!("const x = (if 5 < 3 { 'yes!' } else { 'no!' }); $x");
    assert_eq!(actual.out, "no!");

    let actual =
        nu!("const x = (if 5 < 3 { 'yes!' } else if 4 < 5 { 'no!' } else { 'okay!' }); $x");
    assert_eq!(actual.out, "no!");
}
