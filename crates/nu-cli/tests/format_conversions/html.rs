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
                cd --help | to html --html_color
            "#
        )
    );
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; <span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#037979;'>dirname<span style='color:black;font-weight:normal;'><br><br>  Change to your home directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'><br><br>  Change to your home directory (alternate version)<br>  &gt; </span></span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#037979;'>~<span style='color:black;font-weight:normal;'><br><br>  Change to the previous directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#037979;'>-<span style='color:black;font-weight:normal;'><br><br></body></html></span></span></span>"
    );
}

#[test]
fn test_no_color_flag() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
                cd --help | to html --no_color
            "#
        )
    );
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; cd dirname<br><br>  Change to your home directory<br>  &gt; cd<br><br>  Change to your home directory (alternate version)<br>  &gt; cd ~<br><br>  Change to the previous directory<br>  &gt; cd -<br><br></body></html>"
    );
}

#[test]
fn test_html_color_where_flag_dark_false() {
    let actual = nu!(
        cwd: ".", pipeline(
            r#"
                where --help | to html --html_color
            "#
        )
    );
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Filter table to match the condition.<br><br>Usage:<br>  &gt; where &lt;condition&gt; {flags} <br><br>Parameters:<br>  &lt;condition&gt; the condition that must match<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  List all files in the current directory with sizes greater than 2kb<br>  &gt; <span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>size<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>&gt;<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#c800c8;font-weight:bold;'>2<span style='color:black;font-weight:normal;'></span></span><span style='color:#037979;font-weight:bold;'>kb<span style='color:black;font-weight:normal;'><br><br>  List only the files in the current directory<br>  &gt; </span></span><span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>type<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>==<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:green;'>File<span style='color:black;font-weight:normal;'><br><br>  List all files with names that contain &quot;Car&quot;<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>name<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>=~<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:green;'>&quot;Car&quot;<span style='color:black;font-weight:normal;'><br><br>  List all files that were modified in the last two months<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>modified<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>&lt;=<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#c800c8;font-weight:bold;'>2<span style='color:black;font-weight:normal;'></span></span><span style='color:#037979;font-weight:bold;'>M<span style='color:black;font-weight:normal;'><br><br></body></html></span></span></span>"
    );
}
