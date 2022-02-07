<<<<<<< HEAD
use nu_test_support::pipeline as input;
use nu_test_support::playground::{says, Playground};

use hamcrest2::assert_that;
use hamcrest2::prelude::*;

#[test]
fn adds_a_row_to_the_end() {
    Playground::setup("append_test_1", |_, nu| {
        assert_that!(
            nu.pipeline(&input(
                r#"
                echo  [ "Andrés N. Robalino", "Jonathan Turner", "Yehuda Katz" ] 
                | append "pollo loco"
                | nth 3
                "#
            )),
            says().stdout("pollo loco")
        );
    })
=======
use nu_test_support::{nu, pipeline};

#[test]
fn adds_a_row_to_the_end() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                echo  [ "Andrés N. Robalino", "Jonathan Turner", "Yehuda Katz" ] 
                | append "pollo loco"
                | nth 3
        "#
    ));

    assert_eq!(actual.out, "pollo loco");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
