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
        r"<html><style>body { background-color:white;color:black; }</style><body><table><thead><tr><th>name</th></tr></thead><tbody><tr><td>darren</td></tr></tbody></table></body></html>"
    );
}

#[test]
#[ignore]
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
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br><span style='color:green;'>Usage<span style='color:black;font-weight:normal;'>:<br>  &gt; cd (path) <br><br></span></span><span style='color:green;'>Flags<span style='color:black;font-weight:normal;'>:<br>  </span></span><span style='color:#037979;'>-h</span>,<span style='color:black;font-weight:normal;'> </span><span style='color:#037979;'>--help<span style='color:black;font-weight:normal;'> - Display the help message for this command<br><br></span><span style='color:green;'>Signatures<span style='color:black;font-weight:normal;'>:<br>  &lt;nothing&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br>  &lt;string&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br><br></span></span><span style='color:green;'>Parameters<span style='color:black;font-weight:normal;'>:<br>  (optional) </span></span></span><span style='color:#037979;'>path<span style='color:black;font-weight:normal;'> &lt;</span><span style='color:blue;font-weight:bold;'>directory<span style='color:black;font-weight:normal;'>&gt;: the path to change to<br><br></span></span><span style='color:green;'>Examples<span style='color:black;font-weight:normal;'>:<br>  Change to your home directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span></span><span style='color:#037979;'>~<span style='color:black;font-weight:normal;'><br>  </span></span>...<span style='color:black;font-weight:normal;'><br><br>  Change to a directory via abbreviations<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span><span style='color:#037979;'>d/s/9<span style='color:black;font-weight:normal;'><br>  </span></span>...<span style='color:black;font-weight:normal;'><br><br>  Change to the previous working directory ($OLDPWD)<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span><span style='color:#037979;'>-<span style='color:black;font-weight:normal;'><br>  </span></span>...<span style='color:black;font-weight:normal;'><br><br></body></html></span>"
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
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br>Usage:<br>  &gt; cd (path) <br><br>Flags:<br>  -h, --help - Display the help message for this command<br><br>Signatures:<br>  &lt;nothing&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br>  &lt;string&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br><br>Parameters:<br>  (optional) path &lt;directory&gt;: the path to change to<br><br>Examples:<br>  Change to your home directory<br>  &gt; cd ~<br>  ...<br><br>  Change to a directory via abbreviations<br>  &gt; cd d/s/9<br>  ...<br><br>  Change to the previous working directory ($OLDPWD)<br>  &gt; cd -<br>  ...<br><br></body></html>"
    );
}

#[test]
fn test_list() {
    let actual = nu!(
        cwd: ".",
        r#"to html --list | where name == C64 | get 0 | to nuon"#
    );
    assert_eq!(
        actual.out,
        r##"{name: "C64", black: "#090300", red: "#883932", green: "#55a049", yellow: "#bfce72", blue: "#40318d", purple: "#8b3f96", cyan: "#67b6bd", white: "#ffffff", brightBlack: "#000000", brightRed: "#883932", brightGreen: "#55a049", brightYellow: "#bfce72", brightBlue: "#40318d", brightPurple: "#8b3f96", brightCyan: "#67b6bd", brightWhite: "#f7f7f7", background: "#40318d", foreground: "#7869c4"}"##
    );
}
