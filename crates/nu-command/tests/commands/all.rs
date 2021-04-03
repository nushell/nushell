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
            says().to_stdout("true")
        );
    })
}

#[test]
fn checks_all_columns_of_a_table_is_true() {
    Playground::setup("any_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
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
            )),
            says().to_stdout("true")
        );
    })
}
