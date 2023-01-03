use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn adds_a_row_to_the_beginning() {
    Playground::setup("prepend_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.txt",
            r#"
                Andrés N. Robalino
                Jonathan Turner
                Yehuda Katz
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.txt
                | lines
                | prepend "pollo loco"
                | get 0
                "#
        ));

        assert_eq!(actual.out, "pollo loco");
    })
}
