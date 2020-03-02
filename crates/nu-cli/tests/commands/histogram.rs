use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn summarizes() {
    Playground::setup("histogram_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "los_tres_caballeros.csv",
            r#"
                first_name,last_name,rusty_at
                Andrés,Robalino,Ecuador
                Jonathan,Turner,Estados Unidos
                Yehuda,Katz,Estados Unidos
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open los_tres_caballeros.csv
                | histogram rusty_at countries
                | where rusty_at == "Ecuador"
                | get countries
                | echo $it
            "#
        ));

        assert_eq!(actual, "**************************************************");
        // 50%
    })
}
