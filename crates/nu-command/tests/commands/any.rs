use nu_test_support::nu;

#[test]
fn checks_any_row_is_true() {
    let actual = nu!(r#"
        echo  [ "Ecuador", "USA", "New Zealand" ]
        | any {|it| $it == "New Zealand" }
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_any_column_of_a_table_is_true() {
    let actual = nu!("
        echo [
                [  first_name, last_name,   rusty_at, likes  ];
                [      Andrés,  Robalino, '10/11/2013',   1    ]
                [    JT,    Turner, '10/12/2013',   1    ]
                [      Darren, Schroeder, '10/11/2013',   1    ]
                [      Yehuda,      Katz, '10/11/2013',   1    ]
        ]
        | any {|x| $x.rusty_at == '10/12/2013' }
    ");

    assert_eq!(actual.out, "true");
}

#[test]
fn checks_if_any_returns_error_with_invalid_command() {
    // Using `with-env` to remove `st` possibly being an external program
    let actual = nu!(r#"
        with-env {PATH: ""} {
            [red orange yellow green blue purple] | any {|it| ($it | st length) > 4 }
        }
    "#);

    assert!(
        actual.err.contains("Command `st` not found") && actual.err.contains("Did you mean `ast`?")
    );
}

#[test]
fn works_with_1_param_blocks() {
    let actual = nu!("[1 2 3] | any {|e| print $e | false }");

    assert_eq!(actual.out, "123false");
}

#[test]
fn works_with_0_param_blocks() {
    let actual = nu!("[1 2 3] | any {|| print $in | false }");

    assert_eq!(actual.out, "123false");
}

#[test]
fn early_exits_with_1_param_blocks() {
    let actual = nu!("[1 2 3] | any {|e| print $e | true }");

    assert_eq!(actual.out, "1true");
}

#[test]
fn early_exits_with_0_param_blocks() {
    let actual = nu!("[1 2 3] | any {|| print $in | true }");

    assert_eq!(actual.out, "1true");
}

#[test]
fn any_uses_enumerate_index() {
    let actual = nu!("[7 8 9] | enumerate | any {|el| print $el.index | false }");

    assert_eq!(actual.out, "012false");
}

#[test]
fn unique_env_each_iteration() {
    let actual = nu!(
        cwd: "tests/fixtures/formats",
        "[1 2] | any {|| print ($env.PWD | str ends-with 'formats') | cd '/' | false } | to nuon"
    );

    assert_eq!(actual.out, "truetruefalse");
}
