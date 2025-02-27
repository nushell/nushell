use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::fs;

#[test]
fn def_with_trailing_comma() {
    let actual = nu!("def test-command [ foo: int, ] { $foo }; test-command 1");

    assert!(actual.out == "1");
}

#[test]
fn def_with_comment() {
    Playground::setup("def_with_comment", |dirs, _| {
        let data = r#"
#My echo
export def e [arg] {echo $arg}
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "use def_test e; help e | to json -r"
        );

        assert!(actual.out.contains("My echo\\n\\n"));
    });
}

#[test]
fn def_with_param_comment() {
    Playground::setup("def_with_param_comment", |dirs, _| {
        let data = r#"
export def e [
param:string #My cool attractive param
] {echo $param};
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "use def_test e; help e"
        );

        assert!(actual.out.contains(r#"My cool attractive param"#));
    })
}

#[test]
fn def_errors_with_no_space_between_params_and_name_1() {
    let actual = nu!("def test-command[] {}");

    assert!(actual.err.contains("expected space"));
}

#[test]
fn def_errors_with_no_space_between_params_and_name_2() {
    let actual = nu!("def --env test-command() {}");

    assert!(actual.err.contains("expected space"));
}

#[test]
fn def_errors_with_multiple_short_flags() {
    let actual = nu!("def test-command [ --long(-l)(-o) ] {}");

    assert!(actual.err.contains("expected only one short flag"));
}

#[test]
fn def_errors_with_comma_before_alternative_short_flag() {
    let actual = nu!("def test-command [ --long, (-l) ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_comma_before_equals() {
    let actual = nu!("def test-command [ foo, = 1 ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_colon_before_equals() {
    let actual = nu!("def test-command [ foo: = 1 ] {}");

    assert!(actual.err.contains("expected type"));
}

#[test]
fn def_errors_with_comma_before_colon() {
    let actual = nu!("def test-command [ foo, : int ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_multiple_colons() {
    let actual = nu!("def test-command [ foo::int ] {}");
    assert!(actual.err.contains("expected type"));
}

#[test]
fn def_errors_with_multiple_types() {
    let actual = nu!("def test-command [ foo:int:string ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_trailing_colon() {
    let actual = nu!("def test-command [ foo: int: ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_errors_with_trailing_default_value() {
    let actual = nu!("def test-command [ foo: int = ] {}");

    assert!(actual.err.contains("expected default value"));
}

#[test]
fn def_errors_with_multiple_commas() {
    let actual = nu!("def test-command [ foo,,bar ] {}");

    assert!(actual.err.contains("expected parameter"));
}

#[test]
fn def_fails_with_invalid_name() {
    let err_msg = "command name can't be a number, a filesize, or contain a hash # or caret ^";
    let actual = nu!(r#"def 1234 = echo "test""#);
    assert!(actual.err.contains(err_msg));

    let actual = nu!(r#"def 5gib = echo "test""#);
    assert!(actual.err.contains(err_msg));

    let actual = nu!("def ^foo [] {}");
    assert!(actual.err.contains(err_msg));
}

#[test]
fn def_with_list() {
    Playground::setup("def_with_list", |dirs, _| {
        let data = r#"
def e [
param: list
] {echo $param};
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "source def_test; e [one] | to json -r"
        );

        assert!(actual.out.contains(r#"one"#));
    })
}

#[test]
fn def_with_default_list() {
    Playground::setup("def_with_default_list", |dirs, _| {
        let data = r#"
def f [
param: list = [one]
] {echo $param};
            "#;
        fs::write(dirs.root().join("def_test"), data).expect("Unable to write file");
        let actual = nu!(
            cwd: dirs.root(),
            "source def_test; f | to json -r"
        );

        assert!(actual.out.contains(r#"["one"]"#));
    })
}

#[test]
fn def_with_paren_params() {
    let actual = nu!("def foo (x: int, y: int) { $x + $y }; foo 1 2");

    assert_eq!(actual.out, "3");
}

#[test]
fn def_default_value_shouldnt_restrict_explicit_type() {
    let actual = nu!("def foo [x: any = null] { $x }; foo 1");
    assert_eq!(actual.out, "1");
    let actual2 = nu!("def foo [--x: any = null] { $x }; foo --x 1");
    assert_eq!(actual2.out, "1");
}

#[test]
fn def_default_value_should_restrict_implicit_type() {
    let actual = nu!("def foo [x = 3] { $x }; foo 3.0");
    assert!(actual.err.contains("expected int"));
    let actual2 = nu!("def foo2 [--x = 3] { $x }; foo2 --x 3.0");
    assert!(actual2.err.contains("expected int"));
}

#[test]
fn def_wrapped_with_block() {
    let actual = nu!(
        "def --wrapped foo [...rest] { print ($rest | str join ',' ) }; foo --bar baz -- -q -u -x"
    );

    assert_eq!(actual.out, "--bar,baz,--,-q,-u,-x");
}

#[test]
fn def_wrapped_from_module() {
    let actual = nu!(r#"module spam {
            export def --wrapped my-echo [...rest] { nu --testbin cococo ...$rest }
        }

        use spam
        spam my-echo foo -b -as -9 --abc -- -Dxmy=AKOO - bar
        "#);

    assert!(actual
        .out
        .contains("foo -b -as -9 --abc -- -Dxmy=AKOO - bar"));
}

#[test]
fn def_cursed_env_flag_positions() {
    let actual = nu!("def spam --env [] { $env.SPAM = 'spam' }; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");

    let actual =
        nu!("def spam --env []: nothing -> nothing { $env.SPAM = 'spam' }; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");
}

#[test]
#[ignore = "TODO: Investigate why it's not working, it might be the signature parsing"]
fn def_cursed_env_flag_positions_2() {
    let actual = nu!("def spam [] --env { $env.SPAM = 'spam' }; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");

    let actual = nu!("def spam [] { $env.SPAM = 'spam' } --env; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");

    let actual =
        nu!("def spam []: nothing -> nothing { $env.SPAM = 'spam' } --env; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");
}

#[test]
fn export_def_cursed_env_flag_positions() {
    let actual = nu!("export def spam --env [] { $env.SPAM = 'spam' }; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");

    let actual =
        nu!("export def spam --env []: nothing -> nothing { $env.SPAM = 'spam' }; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");
}

#[test]
#[ignore = "TODO: Investigate why it's not working, it might be the signature parsing"]
fn export_def_cursed_env_flag_positions_2() {
    let actual = nu!("export def spam [] --env { $env.SPAM = 'spam' }; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");

    let actual = nu!("export def spam [] { $env.SPAM = 'spam' } --env; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");

    let actual =
        nu!("export def spam []: nothing -> nothing { $env.SPAM = 'spam' } --env; spam; $env.SPAM");
    assert_eq!(actual.out, "spam");
}

#[test]
fn def_cursed_wrapped_flag_positions() {
    let actual = nu!("def spam --wrapped [...rest] { $rest.0 }; spam --foo");
    assert_eq!(actual.out, "--foo");

    let actual = nu!("def spam --wrapped [...rest]: nothing -> nothing { $rest.0 }; spam --foo");
    assert_eq!(actual.out, "--foo");
}

#[test]
#[ignore = "TODO: Investigate why it's not working, it might be the signature parsing"]
fn def_cursed_wrapped_flag_positions_2() {
    let actual = nu!("def spam [...rest] --wrapped { $rest.0 }; spam --foo");
    assert_eq!(actual.out, "--foo");

    let actual = nu!("def spam [...rest] { $rest.0 } --wrapped; spam --foo");
    assert_eq!(actual.out, "--foo");

    let actual = nu!("def spam [...rest]: nothing -> nothing { $rest.0 } --wrapped; spam --foo");
    assert_eq!(actual.out, "--foo");
}

#[test]
fn def_wrapped_missing_rest_error() {
    let actual = nu!("def --wrapped spam [] {}");
    assert!(actual.err.contains("missing_positional"))
}

#[test]
fn def_wrapped_wrong_rest_type_error() {
    let actual = nu!("def --wrapped spam [...eggs: list<string>] { $eggs }");
    assert!(actual.err.contains("type_mismatch_help"));
    assert!(actual.err.contains("of ...eggs to 'string'"));
}

#[test]
fn def_env_wrapped() {
    let actual = nu!(
        "def --env --wrapped spam [...eggs: string] { $env.SPAM = $eggs.0 }; spam bacon; $env.SPAM"
    );
    assert_eq!(actual.out, "bacon");
}

#[test]
fn def_env_wrapped_no_help() {
    let actual = nu!("def --wrapped foo [...rest] { echo $rest }; foo -h | to json --raw");
    assert_eq!(actual.out, r#"["-h"]"#);
}

#[test]
fn def_recursive_func_should_work() {
    let actual = nu!("def bar [] { let x = 1; ($x | foo) }; def foo [] { foo }");
    assert!(actual.err.is_empty());

    let actual = nu!(r#"
def recursive [c: int] {
    if ($c == 0) { return }
    if ($c mod 2 > 0) {
        $in | recursive ($c - 1)
    } else {
        recursive ($c - 1)
    }
}"#);
    assert!(actual.err.is_empty());
}

#[test]
fn export_def_recursive_func_should_work() {
    let actual = nu!("export def bar [] { let x = 1; ($x | foo) }; export def foo [] { foo }");
    assert!(actual.err.is_empty());

    let actual = nu!(r#"
export def recursive [c: int] {
    if ($c == 0) { return }
    if ($c mod 2 > 0) {
        $in | recursive ($c - 1)
    } else {
        recursive ($c - 1)
    }
}"#);
    assert!(actual.err.is_empty());
}
