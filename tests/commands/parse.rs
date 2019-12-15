use test_support::{nu, pipeline};

#[test]
fn extracts_fields_from_the_given_the_pattern() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open fileA.txt
            | parse "{Name}={Value}"
            | nth 1
            | get Value
            | echo $it
        "#
    ));

    assert_eq!(actual, "StupidLongName");
}
