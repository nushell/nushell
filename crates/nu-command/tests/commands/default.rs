use nu_test_support::{nu, pipeline};

#[test]
fn adds_row_data_if_column_missing() {
    let sample = r#"
                {
                    "amigos": [
                        {"name":   "Yehuda"},
                        {"name": "JT", "rusty_luck": 0},
                        {"name":   "Andres", "rusty_luck": 0},
                        {"name":"GorbyPuff"}
                    ]
                }
            "#;

    let actual = nu!(pipeline(&format!(
        "
                {sample}
                | get amigos
                | default 1 rusty_luck
                | where rusty_luck == 1
                | length
            "
    )));

    assert_eq!(actual.out, "2");
}

#[test]
fn default_after_empty_filter() {
    let actual = nu!("[a b] | where $it == 'c' | get -i 0 | default 'd'");

    assert_eq!(actual.out, "d");
}
