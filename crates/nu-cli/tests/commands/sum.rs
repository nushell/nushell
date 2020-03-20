use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn all() {
    Playground::setup("sum_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "meals.csv",
            r#"
                description,calories
                "1 large egg",90
                "1 cup white rice",250
                "1 tablespoon fish oil",108
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open meals.csv
                | get calories
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual, "448");
    })
}
