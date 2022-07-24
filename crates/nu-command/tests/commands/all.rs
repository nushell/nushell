use nu_test_support::{nu, pipeline};

#[test]
fn checks_all_rows_are_true() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                echo  [ "Andrés", "Andrés", "Andrés" ]
                | all? $it == "Andrés"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_all_rows_are_false_with_param() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                [1, 2, 3, 4] | all? { |a| $a >= 5 }
        "#
    ));

    assert_eq!(actual.out, "false");
}

#[test]
fn checks_all_rows_are_true_with_param() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                [1, 2, 3, 4] | all? { |a| $a < 5 }
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_all_columns_of_a_table_is_true() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                echo [
                        [  first_name, last_name,   rusty_at, likes  ];
                        [      Andrés,  Robalino, 10/11/2013,   1    ]
                        [    Jonathan,    Turner, 10/12/2013,   1    ]
                        [      Darren, Schroeder, 10/11/2013,   1    ]
                        [      Yehuda,      Katz, 10/11/2013,   1    ]
                ]
                | all? likes > 0
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_if_all_returns_error_with_invalid_command() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [red orange yellow green blue purple] | all? ($it | st length) > 4
        "#
    ));

    assert!(actual.err.contains("can't run executable") || actual.err.contains("type_mismatch"));
}
