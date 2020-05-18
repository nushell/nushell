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

#[test]
fn computes_sum_of_individual_row() {
    Playground::setup("sum_test_3", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "calendar.json",
            r#"
                    [
                        {
                            "sunday": null,
                            "monday": null,
                            "tuesday": null,
                            "wednesday": null,
                            "thursday": null,
                            "friday": 1,
                            "saturday": 2
                        },
                        {
                            "sunday": 3,
                            "monday": 4,
                            "tuesday": 5,
                            "wednesday": 6,
                            "thursday": 7,
                            "friday": 8,
                            "saturday": 9
                        },
                        {
                            "sunday": 10,
                            "monday": 11,
                            "tuesday": 12,
                            "wednesday": 13,
                            "thursday": 14,
                            "friday": 15,
                            "saturday": 16
                        },
                        {
                            "sunday": 17,
                            "monday": 18,
                            "tuesday": 19,
                            "wednesday": 20,
                            "thursday": 21,
                            "friday": 22,
                            "saturday": 23
                        },
                        {
                            "sunday": 24,
                            "monday": 25,
                            "tuesday": 26,
                            "wednesday": 27,
                            "thursday": 28,
                            "friday": 29,
                            "saturday": 30
                        },
                        {
                            "sunday": 31,
                            "monday": null,
                            "tuesday": null,
                            "wednesday": null,
                            "thursday": null,
                            "friday": null,
                            "saturday": null
                        }
                    ]
                "#,
        )]);
        let answers_for_rows = ["3", "42", "91", "140", "189", "0"];
        for (row_idx, answer) in answers_for_rows.iter().enumerate() {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                    &format!(r#"
                        open "calendar.json"
                        | nth {}
                        | sum
                    "#, row_idx)
            ));
            assert_eq!(actual.out, *answer);
        }
    })
}

#[test]
fn compute_sum_of_multiple_rows() {
    Playground::setup("sum_test_4", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "calendar.json",
            r#"
                    [
                        {
                            "sunday": null,
                            "monday": null,
                            "tuesday": null,
                            "wednesday": null,
                            "thursday": null,
                            "friday": 1,
                            "saturday": 2
                        },
                        {
                            "sunday": 3,
                            "monday": 4,
                            "tuesday": 5,
                            "wednesday": 6,
                            "thursday": 7,
                            "friday": 8,
                            "saturday": 9
                        },
                        {
                            "sunday": 10,
                            "monday": 11,
                            "tuesday": 12,
                            "wednesday": 13,
                            "thursday": 14,
                            "friday": 15,
                            "saturday": 16
                        },
                        {
                            "sunday": 17,
                            "monday": 18,
                            "tuesday": 19,
                            "wednesday": 20,
                            "thursday": 21,
                            "friday": 22,
                            "saturday": 23
                        },
                        {
                            "sunday": 24,
                            "monday": 25,
                            "tuesday": 26,
                            "wednesday": 27,
                            "thursday": 28,
                            "friday": 29,
                            "saturday": 30
                        },
                        {
                            "sunday": 31,
                            "monday": null,
                            "tuesday": null,
                            "wednesday": null,
                            "thursday": null,
                            "friday": null,
                            "saturday": null
                        }
                    ]
                "#,
        )]);

        let answers_for_rows = ["3", "42", "91", "140", "189", "0"];
        for (row_idx, answer) in answers_for_rows.iter().enumerate() {
            let actual = nu!(
                cwd: dirs.test(), pipeline(
                    &format!(r#"
                        open "calendar.json"
                        | sum
                        | nth {}
                    "#, row_idx)
            ));
            assert_eq!(actual.out, *answer);
        }
    })
}
