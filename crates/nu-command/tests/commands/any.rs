use nu_test_support::{nu, pipeline};

#[test]
fn checks_any_row_is_true() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                echo  [ "Ecuador", "USA", "New Zealand" ]
                | any $it == "New Zealand"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_any_column_of_a_table_is_true() {
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
                | any rusty_at == 10/12/2013
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_if_any_returns_error_with_invalid_command() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [red orange yellow green blue purple] | any ($it | st length) > 4
        "#
    ));

    assert!(actual.err.contains("can't run executable") || actual.err.contains("did you mean"));
}

#[test]
fn works_with_1_param_blocks() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[1 2 3] | any {|e| print $e | false }"#
    ));

    assert_eq!(actual.out, "123false");
}

#[test]
fn works_with_0_param_blocks() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[1 2 3] | any { print $in | false }"#
    ));

    assert_eq!(actual.out, "123false");
}

#[test]
fn early_exits_with_1_param_blocks() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[1 2 3] | any {|e| print $e | true }"#
    ));

    assert_eq!(actual.out, "1true");
}

#[test]
fn early_exits_with_0_param_blocks() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[1 2 3] | any { print $in | true }"#
    ));

    assert_eq!(actual.out, "1true");
}

#[test]
fn unique_env_each_iteration() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "[1 2] | any { print ($env.PWD | str ends-with 'formats') | cd '/' | false } | to nuon"
    );

    assert_eq!(actual.out, "truetruefalse");
}
