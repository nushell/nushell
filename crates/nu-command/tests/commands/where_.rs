use nu_test_support::nu;

#[test]
fn filters_by_unit_size_comparison() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "ls | where size > 1kib | sort-by size | get name | first | str trim"
    );

    assert_eq!(actual.out, "cargo_sample.toml");
}

#[test]
fn filters_with_nothing_comparison() {
    let actual = nu!(
        r#"'[{"foo": 3}, {"foo": null}, {"foo": 4}]' | from json | get foo | compact | where $it > 1 | math sum"#
    );

    assert_eq!(actual.out, "7");
}

#[test]
fn where_inside_block_works() {
    let actual = nu!("{|x| ls | where $it =~ 'foo' } | describe");

    assert_eq!(actual.out, "closure");
}

#[test]
fn it_inside_complex_subexpression() {
    let actual = nu!(r#"1..10 | where [($it * $it)].0 > 40  | to nuon"#);
    assert_eq!(actual.out, r#"[7, 8, 9, 10]"#)
}

#[test]
fn filters_with_0_arity_block() {
    let actual = nu!("[1 2 3 4] | where {|| $in < 3 } | to nuon");

    assert_eq!(actual.out, "[1, 2]");
}

#[test]
fn filters_with_1_arity_block() {
    let actual = nu!("[1 2 3 6 7 8] | where {|e| $e < 5 } | to nuon");

    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn unique_env_each_iteration() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "[1 2] | where {|| print ($env.PWD | str ends-with 'formats') | cd '/' | true } | to nuon"
    );

    assert_eq!(actual.out, "truetrue[1, 2]");
}

#[test]
fn where_in_table() {
    let actual = nu!(
        r#"'[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where name in ["foo"] | get size | math sum"#
    );

    assert_eq!(actual.out, "5");
}

#[test]
fn where_not_in_table() {
    let actual = nu!(
        r#"'[{"name": "foo", "size": 3}, {"name": "foo", "size": 2}, {"name": "bar", "size": 4}]' | from json | where name not-in ["foo"] | get size | math sum"#
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn where_uses_enumerate_index() {
    let actual = nu!("[7 8 9 10] | enumerate | where {|el| $el.index < 2 } | to nuon");

    assert_eq!(actual.out, "[[index, item]; [0, 7], [1, 8]]");
}

#[cfg(feature = "sqlite")]
#[test]
fn binary_operator_comparisons() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first 4
        | where z > 4200
        | get z.0
    ");

    assert_eq!(actual.out, "4253");

    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first 4
        | where z >= 4253
        | get z.0
    ");

    assert_eq!(actual.out, "4253");

    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first 4
        | where z < 10
        | get z.0
    ");

    assert_eq!(actual.out, "1");

    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | first 4
        | where z <= 1
        | get z.0
    ");

    assert_eq!(actual.out, "1");

    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get ints
        | where z != 1
        | first
        | get z
    ");

    assert_eq!(actual.out, "42");
}

#[cfg(feature = "sqlite")]
#[test]
fn contains_operator() {
    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | where x =~ ell
        | length
    ");

    assert_eq!(actual.out, "4");

    let actual = nu!(cwd: "tests/fixtures/formats", "
        open sample.db
        | get strings
        | where x !~ ell
        | length
    ");

    assert_eq!(actual.out, "2");
}

#[test]
fn fail_on_non_iterator() {
    let actual = nu!(r#"{"name": "foo", "size": 3} | where name == "foo""#);

    assert!(actual.err.contains("command doesn't support"));
}

// Test that filtering on columns that might be missing/null works
#[test]
fn where_gt_null() {
    let actual = nu!("[{foo: 123} {}] | where foo? > 10 | to nuon");
    assert_eq!(actual.out, "[[foo]; [123]]");
}

#[test]
fn has_operator() {
    let actual = nu!(
        r#"[[name, children]; [foo, [a, b]], [bar [b, c]], [baz, [c, d]]] | where children has "a" | to nuon"#
    );
    assert_eq!(actual.out, r#"[[name, children]; [foo, [a, b]]]"#);

    let actual = nu!(
        r#"[[name, children]; [foo, [a, b]], [bar [b, c]], [baz, [c, d]]] | where children not-has "a" | to nuon"#
    );
    assert_eq!(
        actual.out,
        r#"[[name, children]; [bar, [b, c]], [baz, [c, d]]]"#
    );

    let actual = nu!(r#"{foo: 1} has foo"#);
    assert_eq!(actual.out, "true");

    let actual = nu!(r#"{foo: 1} has bar "#);
    assert_eq!(actual.out, "false");
}

#[test]
fn stored_condition() {
    let actual = nu!(r#"let cond = { $in mod 2 == 0 }; 1..10 | where $cond | to nuon"#);
    assert_eq!(actual.out, r#"[2, 4, 6, 8, 10]"#)
}

#[test]
fn nested_stored_condition() {
    let actual =
        nu!(r#"let nested = {cond: { $in mod 2 == 0 }}; 1..10 | where $nested.cond | to nuon"#);
    assert_eq!(actual.out, r#"[2, 4, 6, 8, 10]"#)
}
