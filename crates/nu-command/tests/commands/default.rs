use nu_test_support::{nu, pipeline};

#[test]
fn adds_row_data_if_column_missing() {
    let sample = r#"
                {
                    "amigos": [
                        {"name": "Yehuda"},
                        {"name": "JT", "rusty_luck": 0},
                        {"name": "Andres", "rusty_luck": 0},
                        {"name": "Michael", "rusty_luck": []},
                        {"name": "Darren", "rusty_luck": {}},
                        {"name": "Stefan", "rusty_luck": ""},
                        {"name": "GorbyPuff"}
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

#[test]
fn keeps_nulls_in_lists() {
    let actual = nu!(r#"[null, 2, 3] | default [] | to json -r"#);
    assert_eq!(actual.out, "[null,2,3]");
}

#[test]
fn replaces_null() {
    let actual = nu!(r#"null | default 1"#);
    assert_eq!(actual.out, "1");
}

#[test]
fn adds_row_data_if_column_missing_or_empty() {
    let sample = r#"
                {
                    "amigos": [
                        {"name": "Yehuda"},
                        {"name": "JT", "rusty_luck": 0},
                        {"name": "Andres", "rusty_luck": 0},
                        {"name": "Michael", "rusty_luck": []},
                        {"name": "Darren", "rusty_luck": {}},
                        {"name": "Stefan", "rusty_luck": ""},
                        {"name": "GorbyPuff"}
                    ]
                }
            "#;

    let actual = nu!(pipeline(&format!(
        "
                {sample}
                | get amigos
                | default -e 1 rusty_luck
                | where rusty_luck == 1
                | length
            "
    )));

    assert_eq!(actual.out, "5");
}

#[test]
fn replace_empty_string() {
    let actual = nu!(r#"'' | default -e foo"#);
    assert_eq!(actual.out, "foo");
}

#[test]
fn do_not_replace_empty_string() {
    let actual = nu!(r#"'' | default 1"#);
    assert_eq!(actual.out, "");
}

#[test]
fn replace_empty_list() {
    let actual = nu!(r#"[] | default -e foo"#);
    assert_eq!(actual.out, "foo");
}

#[test]
fn do_not_replace_empty_list() {
    let actual = nu!(r#"[] | default 1 | length"#);
    assert_eq!(actual.out, "0");
}

#[test]
fn replace_empty_record() {
    let actual = nu!(r#"{} | default -e foo"#);
    assert_eq!(actual.out, "foo");
}

#[test]
fn do_not_replace_empty_record() {
    let actual = nu!(r#"{} | default {a:5} | columns | length"#);
    assert_eq!(actual.out, "0");
}
