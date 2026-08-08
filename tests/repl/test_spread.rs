use nu_protocol::{ParseError, ShellError};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn spread_in_list() -> Result {
    test().run("[...[]]").expect_value_eq(test_value!([]))?;
    test()
        .run("[1 2 ...[[3] {x: 1}] 5]")
        .expect_value_eq(test_value!([1, 2, [3], { x: 1 }, 5]))?;
    test()
        .run(r#"[...("foo" | split chars) 10]"#)
        .expect_value_eq(test_value!(["f", "o", "o", 10]))?;
    test()
        .run("let l = [1, 2, [3]]; [...$l $l]")
        .expect_value_eq(test_value!([1, 2, [3], [1, 2, [3]]]))?;
    test()
        .run("[ ...[ ...[ ...[ a ] b ] c ] d ]")
        .expect_value_eq(["a", "b", "c", "d"])
}

#[test]
fn not_spread() -> Result {
    test()
        .run("def ... [x] { $x }; ... ...")
        .expect_value_eq("...")?;
    test()
        .run("let a = 4; [... $a ... [1] ... (5) ...bare ...]")
        .expect_value_eq(test_value!([
            "...",
            4,
            "...",
            [1],
            "...",
            5,
            "...bare",
            "..."
        ]))
}

#[test]
fn bad_spread_on_non_list() -> Result {
    let err = test().run("let x = 5; [...$x]").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotSpreadAsList { .. });

    let err = test().run("[...({ x: 1 })]").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotSpreadAsList { .. });
    Ok(())
}

#[test]
fn spread_type_list() -> Result {
    test()
        .run("def f [a: list<int>] { $a | describe }; f [1 ...[]]")
        .expect_value_eq("list<int>")?;
    test()
        .run("def f [a: list<int>] { $a | describe }; f [1 ...[2]]")
        .expect_value_eq("list<int>")?;

    let err = test()
        .run(r#"def f [a: list<int>] { }; f ["foo" ...[4 5 6]]"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "int");

    let err = test()
        .run(r#"def f [a: list<int>] { }; f [1 2 ...["misfit"] 4]"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "int");
    Ok(())
}

#[test]
fn spread_in_record() -> Result {
    test()
        .run("{...{} ...{}, a: 1}")
        .expect_value_eq(test_record! { "a" => 1 })?;
    test()
        .run("{...{...{...{}}}}")
        .expect_value_eq(test_record! {})?;
    test()
        .run("{foo: bar ...{a: {x: 1}} b: 3}")
        .expect_value_eq(test_value!({ foo: "bar", a: { x: 1 }, b: 3 }))
}

#[test]
fn duplicate_cols() -> Result {
    let err = test().run("{a: 1, ...{a: 3}}").expect_shell_error()?;
    assert_matches!(err, ShellError::ColumnDefinedTwice { .. });

    let err = test().run("{...{a: 4, x: 3}, x: 1}").expect_shell_error()?;
    assert_matches!(err, ShellError::ColumnDefinedTwice { .. });

    let err = test()
        .run("{...{a: 0, x: 2}, ...{x: 5}}")
        .expect_shell_error()?;
    assert_matches!(err, ShellError::ColumnDefinedTwice { .. });
    Ok(())
}

#[test]
fn bad_spread_on_non_record() -> Result {
    let err = test().run("let x = 5; { ...$x }").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotSpreadAsRecord { .. });

    let err = test().run("{...([1, 2])}").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotSpreadAsRecord { .. });
    Ok(())
}

#[test]
fn spread_type_record() -> Result {
    test()
        .run("def f [a: record<x: int>] { $a.x }; f { ...{x: 0} }")
        .expect_value_eq(0)?;

    let err = test()
        .run(r#"def f [a: record<x: int>] {}; f { ...{x: "not an int"} }"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(..));
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_COCOCO)]
fn spread_external_args() -> Result {
    test()
        .run(r#"cococo ...[1 "foo"] 2 ...[3 "bar"]"#)
        .expect_value_eq("1 foo 2 3 bar")?;

    // exec doesn't have rest parameters but allows unknown arguments
    test()
        .run(r#"nu -n -c 'exec cococo "foo" ...[5 6]'"#)
        .expect_value_eq("foo 5 6")
}

#[test]
fn spread_internal_args() -> Result {
    let code = r#"
        let list = ["foo" 4]
        def f [a b c? d? ...x] { [$a $b $c $d $x] }
        f 1 2 ...[5 6] 7 ...$list
    "#;
    test()
        .run(code)
        .expect_value_eq(test_value!([1, 2, (), (), [5, 6, 7, "foo", 4]]))?;

    let code = "
        def f [a b c? d? ...x] { [$a $b $c $d $x] }
        f 1 2 3 ...[5 6]
    ";
    test()
        .run(code)
        .expect_value_eq(test_value!([1, 2, 3, (), [5, 6]]))?;

    let code = "
        def f [--flag: int ...x] { [$flag $x] }
        f 2 ...[foo] 4 --flag 5 6 ...[7 8]
    ";
    test()
        .run(code)
        .expect_value_eq(test_value!([5, [2, "foo", 4, 6, 7, 8]]))?;

    let code = "
        def f [a b? --flag: int ...x] { [$a $b $flag $x] }
        f 1 ...[foo] 4 --flag 5 6 ...[7 8]
    ";
    test()
        .run(code)
        .expect_value_eq(test_value!([1, (), 5, ["foo", 4, 6, 7, 8]]))
}

#[test]
fn bad_spread_internal_args() -> Result {
    let code = "
        def f [a b c? d? ...x] { echo $a $b $c $d $x }
        f 1 ...[5 6]
    ";
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::MissingPositional(name, _, _) if name == "b");

    let code = "
        def f [a b?] { echo a b c d }
        f ...[5 6]
    ";
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::UnexpectedSpreadArg(_, _));
    Ok(())
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn spread_non_list_args() -> Result {
    let err = test().run("echo ...(1)").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotSpreadAsList { .. });

    let err = test().run("cococo ...(1)").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotSpreadAsList { .. });
    Ok(())
}

#[test]
fn spread_args_type() -> Result {
    let err = test()
        .run(r#"def f [...x: int] {}; f ...["abc"]"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "int");
    Ok(())
}

#[test]
fn explain_spread_args() -> Result {
    test()
        .run("(explain { || echo ...[1 2] }).cmd_args.0 | select arg_type name type")
        .expect_value_eq(test_table![
            ["arg_type", "name", "type"];
            ["spread", "[1 2]", "list<int>"],
        ])
}

#[test]
#[deps(TESTBIN_MEOW)]
fn disallow_implicit_spread_for_externals() -> Result {
    let err = test().run("^meow [1 2]").expect_shell_error()?;
    assert_matches!(err, ShellError::CannotPassListToExternal { .. });
    Ok(())
}

#[test]
fn respect_shape() -> Result {
    let err = test()
        .run("def foo [...rest] { ...$rest }; foo bar baz")
        .expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { .. });

    let err = test().run("module foo { ...$bar }").expect_parse_error()?;
    assert_matches!(err, ParseError::ExpectedKeyword(_, _));

    test()
        .run(r#"def "...$foo" [] {2}; do { ...$foo }"#)
        .expect_value_eq(2)?;
    test()
        .run(r#"match "...$foo" { ...$foo => 5 }"#)
        .expect_value_eq(5)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn spread_null() -> Result {
    // Spread in list
    test().run("[1, 2, ...(null)]").expect_value_eq([1, 2])?;

    // Spread in record
    test()
        .run("{a: 1, b: 2, ...(null)}")
        .expect_value_eq(test_record! { "a" => 1, "b" => 2 })?;

    // Spread to built-in command's ...rest
    test().run("echo 1 2 ...(null)").expect_value_eq([1, 2])?;

    // Spread to custom command's ...rest
    let code = "
        def foo [...rest] { $rest }
        foo ...(null) 1 2 ...(null) 3
    ";
    test().run(code).expect_value_eq([1, 2, 3])?;

    // Spread to external command's arguments
    test().run("cococo 1 ...(null) 2").expect_value_eq("1 2")?;

    Ok(())
}

#[test]
fn named_flag_null_is_omitted() -> Result {
    // Null named value uses signature default
    let code = "
        def f [--x: int = 5] { $x }
        f --x=(null)
    ";
    test().run(code).expect_value_eq(5)?;

    // Null named value without default is same as omitting the flag
    let code = "
        def f [--x: int] { $x }
        f --x=(null)
    ";
    test().run(code).expect_value_eq(())?;

    // Forwarding an unbound optional flag (null) into another command
    let code = "
        def outer [--preserve: list<string>] {
            inner --preserve=$preserve
        }
        def inner [--preserve: list<string>] {
            $preserve
        }
        outer
    ";
    test().run(code).expect_value_eq(())?;

    // Explicit empty list is distinct from null/omit
    let code = "
        def outer [--preserve: list<string>] {
            inner --preserve=$preserve
        }
        def inner [--preserve: list<string>] {
            $preserve
        }
        outer --preserve=[]
    ";
    test().run(code).expect_value_eq(test_value!([]))?;

    // Null switch value is treated as omitted (false)
    let code = "
        def f [--verbose] { $verbose }
        f --verbose=(null)
    ";
    test().run(code).expect_value_eq(false)?;

    Ok(())
}

#[test]
fn named_flag_null_passed_when_type_allows_nothing() -> Result {
    // oneof with nothing: explicit null is bound; omit still uses default
    let code = "
        def f [--x: oneof<int, nothing> = 5] { $x }
        f --x=(null)
    ";
    test().run(code).expect_value_eq(())?;

    let code = "
        def f [--x: oneof<int, nothing> = 5] { $x }
        f
    ";
    test().run(code).expect_value_eq(5)?;

    let code = "
        def f [--x: oneof<int, nothing> = 5] { $x }
        f --x=3
    ";
    test().run(code).expect_value_eq(3)?;

    // `any` accepts nothing, so null is passed through (not the default)
    let code = "
        def f [--x: any = 5] { $x }
        f --x=(null)
    ";
    test().run(code).expect_value_eq(())?;

    // `nothing` type: null is passed through
    let code = "
        def f [--x: nothing] { $x }
        f --x=(null)
    ";
    test().run(code).expect_value_eq(())?;

    // Record spread: null passes when type allows nothing
    let code = "
        def f [--x: oneof<int, nothing> = 5] { $x }
        f ...{x: null}
    ";
    test().run(code).expect_value_eq(())?;

    // Record spread: null still omits when type does not allow nothing
    let code = "
        def f [--x: int = 5] { $x }
        f ...{x: null}
    ";
    test().run(code).expect_value_eq(5)?;

    Ok(())
}

#[test]
fn named_flag_record_spread() -> Result {
    let code = r#"
        def f [--x: int, --y: string, --verbose] {
            {x: $x, y: $y, verbose: $verbose}
        }
        f ...{x: 1, y: "a", verbose: true}
    "#;
    test().run(code).expect_value_eq(test_value!({
        x: 1,
        y: "a",
        verbose: true
    }))?;

    // Null fields are omitted (defaults / null apply)
    let code = r#"
        def f [--x: int = 9, --y: string, --verbose] {
            {x: $x, y: $y, verbose: $verbose}
        }
        f ...{x: null, y: "hi", verbose: false}
    "#;
    test().run(code).expect_value_eq(test_value!({
        x: 9,
        y: "hi",
        verbose: false
    }))?;

    // Dynamic record variable
    let code = r#"
        def f [--x: int, --y: string] { [$x $y] }
        let flags = {x: 3, y: "z"}
        f ...$flags
    "#;
    test().run(code).expect_value_eq(test_value!([3, "z"]))?;

    // Combine named spread with rest positionals
    let code = "
        def f [--flag: int, ...rest] { {flag: $flag, rest: $rest} }
        f ...{flag: 7} a b
    ";
    test().run(code).expect_value_eq(test_value!({
        flag: 7,
        rest: ["a", "b"]
    }))?;

    // Shadowing-style call: flags record + rest paths
    let code = "
        def wrap [--preserve: list<string>, --recursive, ...rest] {
            inner ...{
                preserve: $preserve
                recursive: $recursive
            } ...$rest
        }
        def inner [--preserve: list<string>, --recursive, ...rest] {
            {preserve: $preserve, recursive: $recursive, rest: $rest}
        }
        wrap src dest
    ";
    test().run(code).expect_value_eq(test_value!({
        preserve: (),
        recursive: false,
        rest: ["src", "dest"]
    }))?;

    let code = "
        def wrap [--preserve: list<string>, --recursive, ...rest] {
            inner ...{
                preserve: $preserve
                recursive: $recursive
            } ...$rest
        }
        def inner [--preserve: list<string>, --recursive, ...rest] {
            {preserve: $preserve, recursive: $recursive, rest: $rest}
        }
        wrap --preserve=[mode] --recursive src dest
    ";
    test().run(code).expect_value_eq(test_value!({
        preserve: ["mode"],
        recursive: true,
        rest: ["src", "dest"]
    }))?;

    Ok(())
}

#[test]
fn named_flag_record_spread_short_flags() -> Result {
    // Short-only valued flag + switch
    let code = "
        def f [-a: int, -b] { {a: $a, b: $b} }
        f ...{a: 3, b: true}
    ";
    test().run(code).expect_value_eq(test_value!({
        a: 3,
        b: true
    }))?;

    // Short-only: null omits when type does not accept nothing
    let code = "
        def f [-a: int = 9] { $a }
        f ...{a: null}
    ";
    test().run(code).expect_value_eq(9)?;

    // Dual long+short: short key works
    let code = "
        def f [--verbose(-v)] { $verbose }
        f ...{v: true}
    ";
    test().run(code).expect_value_eq(true)?;

    // Dual long+short: long key still works
    let code = "
        def f [--verbose(-v)] { $verbose }
        f ...{verbose: true}
    ";
    test().run(code).expect_value_eq(true)?;

    // Unknown short key errors
    let code = "
        def f [-a: int] { $a }
        f ...{z: 1}
    ";
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::Generic(_));

    // Long flag named `a` wins over short `-a` of another flag when key is "a"
    let code = "
        def f [--a: int, -b: int] { {a: $a, b: $b} }
        f ...{a: 1, b: 2}
    ";
    test().run(code).expect_value_eq(test_value!({
        a: 1,
        b: 2
    }))?;

    // Multi short-only: both bind correctly (not only the first)
    let code = "
        def f [-a: int, -b: int] { {a: $a, b: $b} }
        f ...{a: 3, b: 4}
    ";
    test().run(code).expect_value_eq(test_value!({
        a: 3,
        b: 4
    }))?;

    Ok(())
}

#[test]
fn named_flag_record_spread_required_named() -> Result {
    // Builtin with required_named: missing flags still parse-error
    let err = test().run(r#"stor create"#).expect_parse_error()?;
    assert_matches!(err, ParseError::MissingRequiredFlag(..));

    // Static record spread supplies required named flags (no MissingRequiredFlag)
    test()
        .run(r#"stor create ...{table-name: "t_spread_ok", columns: {id: int}} | describe"#)
        .expect_value_eq("SQLiteDatabase")?;

    // Static null alone for a required flag is still missing (omitted at runtime)
    let err = test()
        .run(r#"stor create ...{table-name: null, columns: {id: int}}"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::MissingRequiredFlag(..));

    // Missing key entirely in static record is also missing
    let err = test()
        .run(r#"stor create ...{columns: {id: int}}"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::MissingRequiredFlag(..));

    // Static null + later dynamic spread: may supply required flag at runtime (not parse error)
    test()
        .run(
            r#"
            let more = {table-name: "t_spread_dyn", columns: {id: int}}
            stor create ...{table-name: null} ...$more | describe
            "#,
        )
        .expect_value_eq("SQLiteDatabase")?;

    Ok(())
}

#[test]
fn named_flag_list_spread_before_required_errors() -> Result {
    // Dual-purpose: dynamic list before required positionals must error (not leave them unbound)
    let code = "
        def f [a: string, --x: int, ...rest] { {a: $a, x: $x, rest: $rest} }
        let list = [1]
        f ...$list hello
    ";
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::Generic(_));

    // Null rest-mode before required positionals must also error (not bind nothing to `a`)
    let code = "
        def f [a: string, --x: int, ...rest] { {a: $a, x: $x, rest: $rest} }
        f ...(null) hello
    ";
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::Generic(_));
    Ok(())
}

#[test]
fn named_flag_null_spread_rest_mode() -> Result {
    // Null spread enables rest mode: later positionals go to rest (IR parity)
    let code = "
        def f [a?, ...rest] { {a: $a, rest: $rest} }
        f ...(null) later
    ";
    test().run(code).expect_value_eq(test_value!({
        a: (),
        rest: ["later"]
    }))?;
    Ok(())
}

#[test]
fn named_flag_record_spread_unknown_flag_errors() -> Result {
    let code = "
        def f [--x: int] { $x }
        f ...{x: 1, nope: true}
    ";
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::Generic(_));
    Ok(())
}

#[test]
fn named_flag_record_spread_unknown_allowed() -> Result {
    // allows_unknown_args + a known named flag so record spreads are parse-allowed.
    // Unknown keys are forwarded as rest tokens (not "Unknown flag").
    let code = r#"
        def --wrapped f [--known: int, ...rest] { {known: $known, rest: $rest} }
        f ...{known: 1, extra: true}
    "#;
    test().run(code).expect_value_eq(test_value!({
        known: 1,
        rest: ["--extra"]
    }))?;
    Ok(())
}

#[test]
fn named_flag_list_spread_without_rest_errors() -> Result {
    // Dynamic list on a named-only command must not silently drop the list.
    let code = "
        def f [--x: int] { $x }
        let list = [1]
        f ...$list
    ";
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::Generic(_));

    // Explicit list spread is still a parse error (no rest).
    let err = test()
        .run("def f [--x: int] { $x }; f ...[1]")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::UnexpectedSpreadArg(_, _));
    Ok(())
}

#[test]
fn named_flag_record_spread_type_mismatch_errors() -> Result {
    let code = r#"
        def f [--x: int] { $x }
        f ...{x: "hi"}
    "#;
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::CantConvert { .. });
    Ok(())
}

#[test]
fn named_flag_dynamic_record_before_required_positional() -> Result {
    // Dual-purpose commands: flag record before required positionals is allowed.
    let code = "
        def f [a: string, --x: int, ...rest] { {a: $a, x: $x, rest: $rest} }
        let flags = {x: 7}
        f ...$flags hello more
    ";
    test().run(code).expect_value_eq(test_value!({
        a: "hello",
        x: 7,
        rest: ["more"]
    }))
}

#[test]
fn named_flag_record_spread_without_named_params_is_parse_error() -> Result {
    let err = test()
        .run("def f [] { 1 }; f ...{x: 1}")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::UnexpectedSpreadArg(_, _));
    Ok(())
}
