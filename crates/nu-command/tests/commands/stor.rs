use nu_test_support::nu;

#[test]
fn stor_insert() {
    let actual = nu!(r#"
        stor create --table-name test_table --columns { id: int, value: str };
        stor insert -t test_table --data-record {
            id: 1
            value: "'Initial value'"
        };
        stor open | query db 'select value from test_table' | get 0.value
    "#);

    assert_eq!(actual.out, "'Initial value'");
}

#[test]
fn stor_update_with_quote() {
    let actual = nu!(r#"
        stor create --table-name test_table --columns { id: int, value: str };
        stor insert -t test_table --data-record {
            id: 1
            value: "'Initial value'"
        };
        stor update -t test_table --where-clause 'id = 1' --update-record {
            id: 1
            value: "This didn't work, but should now."
        };
        stor open | query db 'select value from test_table' | get 0.value
    "#);

    assert_eq!(actual.out, "This didn't work, but should now.");
}
