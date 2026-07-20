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
fn disallow_implicit_spread_for_externals() -> Result {
    let err = test().run("^echo [1 2]").expect_shell_error()?;
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
