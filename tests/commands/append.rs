use test_support::{nu, pipeline};

#[test]
fn adds_a_row_to_the_end() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open fileA.txt
            | lines
            | append "testme"
            | nth 3
            | echo $it
        "#
    ));

    assert_eq!(actual, "testme");
}
