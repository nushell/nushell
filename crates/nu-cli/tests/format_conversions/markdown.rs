use nu_test_support::{nu, pipeline};

#[test]
fn out_md_simple() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to-md
        "#
    ));

    assert_eq!(actual, "3");
}

#[test]
fn out_md_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo '{"name": "jason"}' | from-json | to-md
        "#
    ));

    assert_eq!(actual, "|name||-||jason|");
}
