use nu_test_support::nu;

#[test]
fn checks_all_rows_are_true() {
    let actual = nu!(r#"
        echo  [ "Andrés", "Andrés", "Andrés" ]
        | all {|it| $it == "Andrés" }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_all_rows_are_false_with_param() {
    let actual = nu!(" [1, 2, 3, 4] | all { |a| $a >= 5 } ");

    assert_eq!(actual.out, "false");
}

#[test]
fn checks_all_rows_are_true_with_param() {
    let actual = nu!(" [1, 2, 3, 4] | all { |a| $a < 5 } ");

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_all_columns_of_a_table_is_true() {
    let actual = nu!("
        echo [
                [  first_name, last_name,   rusty_at, likes  ];
                [      Andrés,  Robalino, '10/11/2013',   1    ]
                [    JT,    Turner, '10/12/2013',   1    ]
                [      Darren, Schroeder, '10/11/2013',   1    ]
                [      Yehuda,      Katz, '10/11/2013',   1    ]
        ]
        | all {|x| $x.likes > 0 }
    ");

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_if_all_returns_error_with_invalid_command() {
    // Using `with-env` to remove `st` possibly being an external program
    let actual = nu!(r#"
        with-env {PATH: ""} {
            [red orange yellow green blue purple] | all {|it| ($it | st length) > 4 }
        }
    "#);

    assert!(
        actual.err.contains("Command `st` not found") && actual.err.contains("Did you mean `ast`?")
    );
}

#[test]
fn works_with_1_param_blocks() {
    let actual = nu!("[1 2 3] | all {|e| print $e | true }");

    assert_eq!(actual.out, "123true");
}

#[test]
fn works_with_0_param_blocks() {
    let actual = nu!("[1 2 3] | all {|| print $in | true }");

    assert_eq!(actual.out, "123true");
}

#[test]
fn early_exits_with_1_param_blocks() {
    let actual = nu!("[1 2 3] | all {|e| print $e | false }");

    assert_eq!(actual.out, "1false");
}

#[test]
fn early_exits_with_0_param_blocks() {
    let actual = nu!("[1 2 3] | all {|| print $in | false }");

    assert_eq!(actual.out, "1false");
}

#[test]
fn all_uses_enumerate_index() {
    let actual = nu!("[7 8 9] | enumerate | all {|el| print $el.index | true }");

    assert_eq!(actual.out, "012true");
}

#[test]
fn unique_env_each_iteration() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "[1 2] | all {|| print ($env.PWD | str ends-with 'formats') | cd '/' | true } | to nuon"
    );

    assert_eq!(actual.out, "truetruetrue");
}
