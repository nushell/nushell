use nu_test_support::{nu, pipeline};

#[test]
fn out_html_simple() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to-html
        "#
    ));

    assert_eq!(actual, "<html><body>3</body></html>");
}

#[test]
fn out_html_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo '{"name": "jason"}' | from-json | to-html
        "#
    ));

    assert_eq!(
        actual,
        "<html><body><table><tr><th>name</th></tr><tr><td>jason</td></tr></table></body></html>"
    );
}
