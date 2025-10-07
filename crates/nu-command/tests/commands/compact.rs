use nu_test_support::nu;

#[test]
fn discards_rows_where_given_column_is_empty() {
    let sample_json = r#"{
        "amigos": [
            {"name":   "Yehuda", "rusty_luck": 1},
            {"name": "JT", "rusty_luck": 1},
            {"name":   "Andres", "rusty_luck": 1},
            {"name":"GorbyPuff"}
        ]
    }"#;

    let actual = nu!(format!(
        "
            {sample_json}
            | get amigos
            | compact rusty_luck
            | length
        "
    ));

    assert_eq!(actual.out, "3");
}
#[test]
fn discards_empty_rows_by_default() {
    let actual = nu!(r#"
            echo "[1,2,3,14,null]"
            | from json
            | compact
            | length
        "#);

    assert_eq!(actual.out, "4");
}

#[test]
fn discard_empty_list_in_table() {
    let actual = nu!(r#"
       [["a", "b"]; ["c", "d"], ["h", []]] | compact -e b | length
    "#);

    assert_eq!(actual.out, "1");
}

#[test]
fn discard_empty_record_in_table() {
    let actual = nu!(r#"
       [["a", "b"]; ["c", "d"], ["h", {}]] | compact -e b | length
    "#);

    assert_eq!(actual.out, "1");
}

#[test]
fn dont_discard_empty_record_in_table_if_column_not_set() {
    let actual = nu!(r#"
       [["a", "b"]; ["c", "d"], ["h", {}]] | compact -e | length
    "#);

    assert_eq!(actual.out, "2");
}

#[test]
fn dont_discard_empty_list_in_table_if_column_not_set() {
    let actual = nu!(r#"
       [["a", "b"]; ["c", "d"], ["h", []]] | compact -e | length
    "#);

    assert_eq!(actual.out, "2");
}

#[test]
fn dont_discard_null_in_table_if_column_not_set() {
    let actual = nu!(r#"
       [["a", "b"]; ["c", "d"], ["h", null]] | compact -e | length
    "#);

    assert_eq!(actual.out, "2");
}
