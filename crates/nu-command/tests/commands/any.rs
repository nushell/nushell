<<<<<<< HEAD
use nu_test_support::pipeline as input;
use nu_test_support::playground::{says, Playground};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn checks_any_row_is_true() {
    Playground::setup("any_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
                r#"
                echo  [ "Ecuador", "USA", "New Zealand" ] 
                | any? $it == "New Zealand"
                "#
            )),
            says().stdout("true")
        );
    })
=======
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn checks_any_column_of_a_table_is_true() {
<<<<<<< HEAD
    Playground::setup("any_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
                r#"
=======
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                echo [
                        [  first_name, last_name,   rusty_at, likes  ];
                        [      Andrés,  Robalino, 10/11/2013,   1    ]
                        [    Jonathan,    Turner, 10/12/2013,   1    ]
                        [      Darren, Schroeder, 10/11/2013,   1    ]
                        [      Yehuda,      Katz, 10/11/2013,   1    ]
                ]
                | any? rusty_at == 10/12/2013
<<<<<<< HEAD
                "#
            )),
            says().stdout("true")
        );
    })
=======
        "#
    ));

    assert_eq!(actual.out, "true");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
