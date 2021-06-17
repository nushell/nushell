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
}
