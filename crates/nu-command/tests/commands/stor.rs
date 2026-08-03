use nu_test_support::prelude::*;

#[test]
fn stor_insert() -> Result {
    let code = r#"
        stor create --table-name stor_insert_table --columns { id: int, value: str };
        stor insert -t stor_insert_table --data-record {
            id: 1
            value: "'Initial value'"
        };
        stor open | query db 'select value from stor_insert_table' | get 0.value
    "#;

    test().run(code).expect_value_eq("'Initial value'")
}

#[test]
fn stor_update_with_quote() -> Result {
    let code = r#"
        stor create --table-name stor_update_table --columns { id: int, value: str };
        stor insert -t stor_update_table --data-record {
            id: 1
            value: "'Initial value'"
        };
        stor update -t stor_update_table --where-clause 'id = 1' --update-record {
            id: 1
            value: "This didn't work, but should now."
        };
        stor open | query db 'select value from stor_update_table' | get 0.value
    "#;

    test()
        .run(code)
        .expect_value_eq("This didn't work, but should now.")
}
