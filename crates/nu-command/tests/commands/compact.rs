use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn discards_rows_where_given_column_is_empty() {
    Playground::setup("compact_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.json",
            r#"
                {
                    "amigos": [
                        {"name":   "Yehuda", "rusty_luck": 1},
                        {"name": "Jonathan", "rusty_luck": 1},
                        {"name":   "Andres", "rusty_luck": 1},
                        {"name":"GorbyPuff"}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_amigos.json
                | get amigos
                | compact rusty_luck
                | length
            "#
        ));

        assert_eq!(actual.out, "3");
    });
}
#[test]
fn discards_empty_rows_by_default() {
    Playground::setup("compact_test_2", |dirs, _| {
        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                echo "[1,2,3,14,null]"
                | from json
                | compact
                | length
            "#
        ));

        assert_eq!(actual.out, "4");
    });
}
