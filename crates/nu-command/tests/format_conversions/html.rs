use nu_test_support::{nu, pipeline};

#[test]
fn out_html_simple() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to html
        "#
    ));

    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>3</body></html>"
    );
}

#[test]
fn out_html_partial() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo 3 | to html -p
        "#
    ));

    assert_eq!(
        actual.out,
        "<div style=\"background-color:white;color:black;\">3</div>"
    );
}

#[test]
fn out_html_table() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            echo '{"name": "darren"}' | from json | to html
        "#
    ));

    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body><table><tr><th>name</th></tr><tr><td>darren</td></tr></table></body></html>"
    );
}

#[test]
fn test_cd_html_color_flag_dark_false() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
                cd --help | to html --html-color
            "#
        )
    );
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br>Usage:<br>  &gt; cd (path) <br><br>Flags:<br>  -h, --help<br>      Display this help message<br><br>Parameters:<br>  (optional) path: the path to change to<br><br></body></html>"
    );
}

#[test]
fn test_no_color_flag() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
                cd --help | to html --no-color
            "#
        )
    );
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br>Usage:<br>  &gt; cd (path) <br><br>Flags:<br>  -h, --help<br>      Display this help message<br><br>Parameters:<br>  (optional) path: the path to change to<br><br></body></html>"
    );
}

#[test]
fn test_html_color_where_flag_dark_false() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
                where --help | to html --html-color
            "#
        )
    );
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Filter values based on a condition.<br><br>Usage:<br>  &gt; where &lt;cond&gt; <br><br>Flags:<br>  -h, --help<br>      Display this help message<br><br>Parameters:<br>  cond: condition<br><br></body></html>"
    );
}
