use nu_test_support::{nu, pipeline};

#[test]
fn checks_any_row_is_true() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                echo  [ "Ecuador", "USA", "New Zealand" ]
                | any? $it == "New Zealand"
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
                | any? rusty_at == 10/12/2013
        "#
    ));

    assert_eq!(actual.out, "true");
}
