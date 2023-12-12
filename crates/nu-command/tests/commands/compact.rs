use nu_test_support::{nu, pipeline};

#[test]
fn discards_rows_where_given_column_is_empty() {
    let sample_json = r#"
                {
                    "amigos": [
                        {"name":   "Yehuda", "rusty_luck": 1},
                        {"name": "JT", "rusty_luck": 1},
                        {"name":   "Andres", "rusty_luck": 1},
                        {"name":"GorbyPuff"}
                    ]
                }
            "#;

    let actual = nu!(pipeline(&format!(
        "
                {sample_json}
                | get amigos
                | compact rusty_luck
                | length
            "
    )));

    assert_eq!(actual.out, "3");
}
#[test]
fn discards_empty_rows_by_default() {
    let actual = nu!(pipeline(
        r#"
                echo "[1,2,3,14,null]"
                | from json
                | compact
                | length
            "#
    ));

    assert_eq!(actual.out, "4");
}
