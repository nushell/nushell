use nu_test_support::{nu, pipeline};

#[test]
fn adds_a_row_to_the_end() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
                echo  [ "Andrés N. Robalino", "Jonathan Turner", "Yehuda Katz" ]
                | append "pollo loco"
                | get 3
        "#
    ));

    assert_eq!(actual.out, "pollo loco");
}
