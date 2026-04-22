use nu_test_support::prelude::*;
#[cfg(feature = "sqlite")]
use rstest::rstest;

#[test]
fn filters_by_unit_size_comparison() -> Result {
    let file: String = test()
        .cwd("tests/fixtures/formats")
        .run("ls | where size > 1kib | sort-by size | get name | first | str trim")?;
    assert_contains("cargo_sample.toml", file);
    Ok(())
}

#[test]
fn filters_with_nothing_comparison() -> Result {
    let code = r#"
        '[{"foo": 3}, {"foo": null}, {"foo": 4}]' 
        | from json 
        | get foo 
        | compact 
        | where $it > 1 
        | math sum
    "#;

    test().run(code).expect_value_eq(7)
}

#[test]
fn where_inside_block_works() -> Result {
    test()
        .run("{|x| ls | where $it =~ 'foo' } | describe")
        .expect_value_eq("closure")
}

#[test]
fn it_inside_complex_subexpression() -> Result {
    test()
        .run("1..10 | where [($it * $it)].0 > 40")
        .expect_value_eq([7, 8, 9, 10])
}

#[test]
fn filters_with_0_arity_block() -> Result {
    test()
        .run("[1 2 3 4] | where {|| $in < 3 }")
        .expect_value_eq([1, 2])
}

#[test]
fn filters_with_1_arity_block() -> Result {
    test()
        .run("[1 2 3 6 7 8] | where {|e| $e < 5 }")
        .expect_value_eq([1, 2, 3])
}

#[test]
fn unique_env_each_iteration() -> Result {
    let code = "
        [1 2] 
        | each {|| let ok = ($env.PWD | str ends-with 'formats'); cd '/'; $ok }
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq([true, true])
}

#[test]
fn where_in_table() -> Result {
    let code = r#"
        '[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]'
        | from json
        | where name in ["foo"]
        | get size
        | math sum
    "#;

    test().run(code).expect_value_eq(5)
}

#[test]
fn where_not_in_table() -> Result {
    let code = r#"
        '[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]'
        | from json
        | where name not-in ["foo"]
        | get size
        | math sum
    "#;

    test().run(code).expect_value_eq(4)
}

#[test]
fn where_uses_enumerate_index() -> Result {
    test()
        .run("[7 8 9 10] | enumerate | where {|el| $el.index < 2 } | to nuon")
        .expect_value_eq("[[index, item]; [0, 7], [1, 8]]")
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case("z > 4200", 4253)]
#[case("z >= 4253", 4253)]
#[case("z < 10", 1)]
#[case("z <= 1", 1)]
#[case("z != 1", 42)]
fn binary_operator_comparison(#[case] op: &str, #[case] value: impl IntoValue) -> Result {
    let code = format!("open sample.db | get ints | first 4 | where {op} | get z.0");

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(value)
}

#[cfg(feature = "sqlite")]
#[rstest]
#[case("x =~ ell", 4)]
#[case("x !~ ell", 2)]
fn contains_operator(#[case] op: &str, #[case] value: impl IntoValue) -> Result {
    let code = format!("open sample.db | get strings | where {op} | length");
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(value)
}

#[test]
fn fail_on_non_iterator() -> Result {
    let err = test()
        .run(r#"{"name": "foo", "size": 3} | where name == "foo""#)
        .expect_parse_error()?;
    assert!(matches!(err, ParseError::InputMismatch { .. }));
    Ok(())
}

// Test that filtering on columns that might be missing/null works
#[test]
fn where_gt_null() -> Result {
    test()
        .run("[{foo: 123} {}] | where foo? > 10 | to nuon")
        .expect_value_eq("[[foo]; [123]]")
}

#[test]
fn has_operator() -> Result {
    let code = r#"
        [[name, children]; [foo, [a, b]], [bar [b, c]], [baz, [c, d]]] 
        | where children has "a" 
        | to nuon
    "#;
    test()
        .run(code)
        .expect_value_eq("[[name, children]; [foo, [a, b]]]")?;

    let code = r#"
        [[name, children]; [foo, [a, b]], [bar [b, c]], [baz, [c, d]]] 
        | where children not-has "a" 
        | to nuon
    "#;
    test()
        .run(code)
        .expect_value_eq("[[name, children]; [bar, [b, c]], [baz, [c, d]]]")?;

    test().run("{foo: 1} has foo").expect_value_eq(true)?;
    test().run("{foo: 1} has bar ").expect_value_eq(false)
}

#[test]
fn stored_condition() -> Result {
    test()
        .run("let cond = { $in mod 2 == 0 }; 1..10 | where $cond")
        .expect_value_eq([2, 4, 6, 8, 10])
}

#[test]
fn nested_stored_condition() -> Result {
    test()
        .run("let nested = {cond: { $in mod 2 == 0 }}; 1..10 | where $nested.cond")
        .expect_value_eq([2, 4, 6, 8, 10])
}
