use nu_test_support::{nu, pipeline};

#[test]
fn out_html_simple() {
    let actual = nu!(pipeline(
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
fn out_html_metadata() {
    let actual = nu!(r#"
            echo 3 | to html | metadata | get content_type
        "#);

    assert_eq!(actual.out, r#"text/html; charset=utf-8"#);
}

#[test]
fn out_html_partial() {
    let actual = nu!(r#"
            echo 3 | to html -p
        "#);

    assert_eq!(
        actual.out,
        "<div style=\"background-color:white;color:black;\">3</div>"
    );
}

#[test]
fn out_html_table() {
    let actual = nu!(r#"
            echo '{"name": "darren"}' | from json | to html
        "#);

    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body><table><thead><tr><th>name</th></tr></thead><tbody><tr><td>darren</td></tr></tbody></table></body></html>"
    );
}

#[test]
#[ignore]
fn test_cd_html_color_flag_dark_false() {
    let actual = nu!(r#"
                cd --help | to html --html-color
            "#);
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br><span style='color:green;'>Usage<span style='color:black;font-weight:normal;'>:<br>  &gt; cd (path) <br><br></span></span><span style='color:green;'>Flags<span style='color:black;font-weight:normal;'>:<br>  </span></span><span style='color:#037979;'>-h</span>,<span style='color:black;font-weight:normal;'> </span><span style='color:#037979;'>--help<span style='color:black;font-weight:normal;'> - Display the help message for this command<br><br></span><span style='color:green;'>Signatures<span style='color:black;font-weight:normal;'>:<br>  &lt;nothing&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br>  &lt;string&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br><br></span></span><span style='color:green;'>Parameters<span style='color:black;font-weight:normal;'>:<br>  (optional) </span></span></span><span style='color:#037979;'>path<span style='color:black;font-weight:normal;'> &lt;</span><span style='color:blue;font-weight:bold;'>directory<span style='color:black;font-weight:normal;'>&gt;: the path to change to<br><br></span></span><span style='color:green;'>Examples<span style='color:black;font-weight:normal;'>:<br>  Change to your home directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span></span><span style='color:#037979;'>~<span style='color:black;font-weight:normal;'><br><br>  Change to a directory via abbreviations<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span><span style='color:#037979;'>d/s/9<span style='color:black;font-weight:normal;'><br><br>  Change to the previous working directory ($OLDPWD)<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span><span style='color:#037979;'>-<span style='color:black;font-weight:normal;'><br><br></body></html></span></span>"
    );
}

#[test]
#[ignore]
fn test_no_color_flag() {
    // TODO replace with something potentially more stable, otherwise this test needs to be
    // manually updated when ever the help output changes
    let actual = nu!(r#"
                cd --help | to html --no-color
            "#);
    assert_eq!(
        actual.out,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br>Usage:<br>  &gt; cd (path) <br><br>Flags:<br>  -h, --help - Display the help message for this command<br><br>Parameters:<br>  path &lt;directory&gt;: The path to change to. (optional)<br><br>Input&#x2f;output types:<br>  ╭─#─┬──input──┬─output──╮<br>  │ 0 │ nothing │ nothing │<br>  │ 1 │ string  │ nothing │<br>  ╰───┴─────────┴─────────╯<br><br>Examples:<br>  Change to your home directory<br>  &gt; cd ~<br><br>  Change to the previous working directory ($OLDPWD)<br>  &gt; cd -<br><br></body></html>"
    )
}

#[test]
fn test_list() {
    let actual = nu!(r#"to html --list | where name == C64 | get 0 | to nuon"#);
    assert_eq!(
        actual.out,
        r##"{name: "C64", black: "#090300", red: "#883932", green: "#55a049", yellow: "#bfce72", blue: "#40318d", purple: "#8b3f96", cyan: "#67b6bd", white: "#ffffff", brightBlack: "#000000", brightRed: "#883932", brightGreen: "#55a049", brightYellow: "#bfce72", brightBlue: "#40318d", brightPurple: "#8b3f96", brightCyan: "#67b6bd", brightWhite: "#f7f7f7", background: "#40318d", foreground: "#7869c4"}"##
    );
}
