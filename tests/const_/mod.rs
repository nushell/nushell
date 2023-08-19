use nu_test_support::nu;
use pretty_assertions::assert_eq;

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
fn const_unsupported() {
    let inp = &["const x = ('abc' | str length)"];

    let actual = nu!(&inp.join("; "));

    assert!(actual.err.contains("not_a_constant"));
}

#[test]
fn const_in_scope() {
    let inp = &["do { const x = 'x'; $x }"];

    let actual = nu!(&inp.join("; "));

    assert_eq!(actual.out, "x");
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
