use nu_test_support::nu;

#[test]
fn adds_a_row_to_the_end() {
    let actual = nu!(r#"
            echo  [ "Andrés N. Robalino", "JT Turner", "Yehuda Katz" ]
            | append "pollo loco"
            | get 3
    "#);

    assert_eq!(actual.out, "pollo loco");
}
