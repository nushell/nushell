use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::playground::Playground;
use nu_test_support::{nu, pipeline};

#[test]
fn all() {
    Playground::setup("sum_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "meals.json",
            r#"
                {
                    meals: [
                        {description: "1 large egg", calories: 90},
                        {description: "1 cup white rice", calories: 250},
                        {description: "1 tablespoon fish oil", calories: 108}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                open meals.json
                | get meals
                | get calories
                | sum
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "448");
    })
}

#[test]
fn outputs_zero_with_no_input() {
    Playground::setup("sum_test_2", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "meals.json",
            r#"
                {
                    meals: [
                        {description: "1 large egg", calories: 90},
                        {description: "1 cup white rice", calories: 250},
                        {description: "1 tablespoon fish oil", calories: 108}
                    ]
                }
            "#,
        )]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                sum
                | echo $it
            "#
        ));

        assert_eq!(actual.out, "0");
    })
}
