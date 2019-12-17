use nu_test_support::{nu, pipeline};

#[test]
fn adds_a_row_to_the_beginning() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open fileA.txt
            | lines
            | prepend "testme"
            | nth 0
            | echo $it
        "#
    ));

    assert_eq!(actual, "testme");
}
