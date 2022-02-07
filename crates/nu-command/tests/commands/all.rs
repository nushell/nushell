<<<<<<< HEAD
use nu_test_support::pipeline as input;
use nu_test_support::playground::{says, Playground};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn checks_all_rows_are_true() {
    Playground::setup("all_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
                r#"
                echo  [ "Andrés", "Andrés", "Andrés" ] 
                | all? $it == "Andrés"
                "#
            )),
            says().stdout("true")
        );
    })
=======
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
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn checks_all_rows_are_false_with_param() {
<<<<<<< HEAD
    Playground::setup("all_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
                r#"
                [1, 2, 3, 4] | all? { |a| $a >= 5 }
                "#
            )),
            says().stdout("false")
        );
    })
=======
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                [1, 2, 3, 4] | all? { |a| $a >= 5 }
        "#
    ));

    assert_eq!(actual.out, "false");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn checks_all_rows_are_true_with_param() {
<<<<<<< HEAD
    Playground::setup("all_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
                r#"
                [1, 2, 3, 4] | all? { |a| $a < 5 }
                "#
            )),
            says().stdout("true")
        );
    })
=======
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                [1, 2, 3, 4] | all? { |a| $a < 5 }
        "#
    ));

    assert_eq!(actual.out, "true");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn checks_all_columns_of_a_table_is_true() {
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
                | all? likes > 0
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
