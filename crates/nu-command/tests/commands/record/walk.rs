use nu_test_support::prelude::*;

#[test]
fn paths_lead_back_to_each_leaf() -> Result {
    let code = r#"
        let data = {a: {"b.c": [10, {"d e": 20}]}, f: 30}
        $data | record walk {|value, path| ($data | get $path) == $value }
    "#;

    test().run(code).expect_value_eq(test_value!({
        a: {"b.c": [true, {"d e": true}]},
        f: true
    }))
}

#[test]
fn path_is_a_cell_path() -> Result {
    test()
        .run("{a: [1]} | record walk {|value, path| $path | describe}")
        .expect_value_eq(test_value!({a: ["cell-path"]}))
}

#[test]
fn value_is_also_input() -> Result {
    test()
        .run("{a: 1, b: {c: 2}} | record walk { $in + 1 }")
        .expect_value_eq(test_value!({a: 2, b: {c: 3}}))
}

#[test]
fn empty_containers_are_kept() -> Result {
    test()
        .run("{a: {}, b: [], c: 1} | record walk {|value| 0}")
        .expect_value_eq(test_value!({a: {}, b: [], c: 0}))
}

#[test]
fn containers_see_updated_children_and_skip_root() -> Result {
    let code = "
        {a: {b: 1}}
        | record walk --containers {|value|
            if ($value | describe) == int { $value + 1 } else { $value | insert seen true }
        }
    ";

    test()
        .run(code)
        .expect_value_eq(test_value!({a: {b: 2, seen: true}}))
}

#[test]
fn containers_include_empty_containers() -> Result {
    test()
        .run("{a: {}, b: []} | record walk --containers {|value, path| $path | to text}")
        .expect_value_eq(test_value!({a: "$.a", b: "$.b"}))
}

#[test]
fn no_lists_passes_lists_whole() -> Result {
    test()
        .run("{a: [1, 2], b: {c: [[3]]}} | record walk --no-lists {|value| $value | length}")
        .expect_value_eq(test_value!({a: 2, b: {c: 1}}))
}

#[test]
fn closure_error_is_returned() -> Result {
    let err = test()
        .run("{a: {b: 1}} | record walk {|value| error make {msg: boom}}")
        .expect_labeled_error()?;
    assert_eq!(err.msg, "boom");
    Ok(())
}

#[test]
fn streams_a_table() -> Result {
    // `1..` never ends, so this only finishes if `record walk` streams.
    let code = "1.. | each {|i| {a: {b: $i}}} | record walk {|value| $value * 10} | first 2";

    test()
        .run(code)
        .expect_value_eq(test_value!([{a: {b: 10}}, {a: {b: 20}}]))
}

#[test]
fn row_after_a_failed_row_gets_its_own_paths() -> Result {
    let code = "
        [{a: {b: x}} {c: 1}]
        | record walk {|value, path| if $value == x { error make {msg: boom} } else { $path | to text } }
        | last
    ";

    test().run(code).expect_value_eq(test_value!({c: "$.c"}))
}
