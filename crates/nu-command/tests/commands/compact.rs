use nu_test_support::prelude::*;

#[test]
fn discards_rows_where_given_column_is_empty() -> Result {
    #[derive(Debug, IntoValue)]
    struct Amigo {
        name: &'static str,
        rusty_luck: Option<u32>,
    }

    #[derive(Debug, IntoValue)]
    struct Input {
        amigos: Vec<Amigo>,
    }

    #[rustfmt::skip]
    let input = Input {
        amigos: vec![
        Amigo { name: "Yehuda",    rusty_luck: Some(1) },
        Amigo { name: "JT",        rusty_luck: Some(1) },
        Amigo { name: "Andres",    rusty_luck: Some(1) },
        Amigo { name: "GorbyPuff", rusty_luck: None    },
    ]};

    let code = "
        $in
        | get amigos
        | compact rusty_luck
        | length
    ";

    test().run_with_data(code, input).expect_value_eq(3)
}
#[test]
fn discards_empty_rows_by_default() -> Result {
    let code = "
        $in
        | compact
        | length
    ";

    test()
        .run_with_data(code, (1, 2, 3, 14, ()))
        .expect_value_eq(4)
}

#[test]
fn discard_empty_list_in_table() -> Result {
    let code = r#"
       [["a", "b"]; ["c", "d"], ["h", []]] 
       | compact -e b 
       | length
    "#;

    test().run(code).expect_value_eq(1)
}

#[test]
fn discard_empty_record_in_table() -> Result {
    let code = r#"
       [["a", "b"]; ["c", "d"], ["h", {}]] 
       | compact -e b 
       | length
    "#;

    test().run(code).expect_value_eq(1)
}

#[test]
fn dont_discard_empty_record_in_table_if_column_not_set() -> Result {
    let code = r#"
       [["a", "b"]; ["c", "d"], ["h", {}]] 
       | compact -e 
       | length
    "#;

    test().run(code).expect_value_eq(2)
}

#[test]
fn dont_discard_empty_list_in_table_if_column_not_set() -> Result {
    let code = r#"
       [["a", "b"]; ["c", "d"], ["h", []]] 
       | compact -e 
       | length
    "#;

    test().run(code).expect_value_eq(2)
}

#[test]
fn dont_discard_null_in_table_if_column_not_set() -> Result {
    let code = r#"
       [["a", "b"]; ["c", "d"], ["h", null]] 
       | compact -e 
       | length
    "#;

    test().run(code).expect_value_eq(2)
}

#[test]
fn keeps_rows_where_nested_cell_path_is_not_null() -> Result {
    let code = "
        [{labels: {c: 1}} {labels: {c: null}}]
        | compact labels.c
        | length
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn discards_rows_where_nested_cell_path_is_empty_with_empty_flag() -> Result {
    let code = r#"
        [{a: {b: ""}} {a: {b: []}} {a: {b: {}}} {a: {b: 1}}]
        | compact -e a.b
        | length
    "#;

    test().run(code).expect_value_eq(1)
}

#[test]
fn discards_rows_where_nested_cell_path_is_missing() -> Result {
    let code = "
        [{a: {b: 1}} {a: {}} {x: 2}]
        | compact a.b
        | length
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn compacts_on_several_nested_cell_paths_at_once() -> Result {
    let code = "
        [{a: {b: 1}, c: 2} {a: {b: null}, c: 3} {a: {b: 4}, c: null}]
        | compact a.b c
        | length
    ";

    test().run(code).expect_value_eq(1)
}

#[test]
fn top_level_column_still_compacts() -> Result {
    let code = "
        [{a: 1} {a: null} {a: 3}]
        | compact a
        | length
    ";

    test().run(code).expect_value_eq(2)
}

#[test]
fn quoted_dotted_name_still_matches_literal_key() -> Result {
    // A quoted name is one cell path member, so `"a.b"` keeps matching the literal key.
    let code = r#"
        [{a: 1, "a.b": null} {a: 2, "a.b": 3}]
        | compact "a.b"
        | length
    "#;

    test().run(code).expect_value_eq(1)
}
