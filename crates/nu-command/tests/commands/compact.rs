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
