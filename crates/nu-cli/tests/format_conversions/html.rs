use nu_test_support::{nu, pipeline};

#[test]
fn out_html_simple() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to html
        "#
    ));

    assert_eq!(actual.out, "<html><style>body { background-color:white;color:black; }</style><body>3</body></html>");
}

#[test]
fn out_html_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo '{"name": "jason"}' | from json | to html
        "#
    ));

    assert_eq!(
        actual.out,
        "<html><style>body { background-color:white;color:black; }</style><body><table style=\"background-color:white;color:black;\"><tr><th>name</th></tr><tr><td>jason</td></tr></table></body></html>"
    );
}
