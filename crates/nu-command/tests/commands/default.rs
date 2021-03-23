use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn adds_row_data_if_column_missing() {
    Playground::setup("default_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_amigos.json",
            r#"
                {
                    "amigos": [
                        {"name":   "Yehuda"},
                        {"name": "Jonathan", "rusty_luck": 0},
                        {"name":   "Andres", "rusty_luck": 0},
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
                | default rusty_luck 1
                | where rusty_luck == 1
                | length
            "#
        ));

        assert_eq!(actual.out, "2");
    });
}
