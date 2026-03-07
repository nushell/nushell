use nu_test_support::prelude::*;
use std::collections::HashMap;

#[test]
fn out_html_simple() -> Result {
    let code = "echo 3 | to html";
    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome,
        r"<html><style>body { background-color:white;color:black; }</style><body>3</body></html>"
    );
    Ok(())
}

#[test]
fn out_html_metadata() -> Result {
    let code = "echo 3 | to html | metadata | get content_type";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "text/html; charset=utf-8");
    Ok(())
}

#[test]
fn out_html_partial() -> Result {
    let code = "echo 3 | to html -p";
    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome,
        "<div style=\"background-color:white;color:black;\">3</div>"
    );
    Ok(())
}

#[test]
fn out_html_table() -> Result {
    let code = r#"echo '{"name": "darren"}' | from json | to html"#;
    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome,
        r"<html><style>body { background-color:white;color:black; }</style><body><table><thead><tr><th>name</th></tr></thead><tbody><tr><td>darren</td></tr></tbody></table></body></html>"
    );
    Ok(())
}

#[test]
#[ignore]
fn test_cd_html_color_flag_dark_false() -> Result {
    let code = "cd --help | to html --html-color";
    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br><span style='color:green;'>Usage<span style='color:black;font-weight:normal;'>:<br>  &gt; cd (path) <br><br></span></span><span style='color:green;'>Flags<span style='color:black;font-weight:normal;'>:<br>  </span></span><span style='color:#037979;'>-h</span>,<span style='color:black;font-weight:normal;'> </span><span style='color:#037979;'>--help<span style='color:black;font-weight:normal;'> - Display the help message for this command<br><br></span><span style='color:green;'>Signatures<span style='color:black;font-weight:normal;'>:<br>  &lt;nothing&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br>  &lt;string&gt; | cd &lt;string?&gt; -&gt; &lt;nothing&gt;<br><br></span></span><span style='color:green;'>Parameters<span style='color:black;font-weight:normal;'>:<br>  (optional) </span></span></span><span style='color:#037979;'>path<span style='color:black;font-weight:normal;'> &lt;</span><span style='color:blue;font-weight:bold;'>directory<span style='color:black;font-weight:normal;'>&gt;: the path to change to<br><br></span></span><span style='color:green;'>Examples<span style='color:black;font-weight:normal;'>:<br>  Change to your home directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span></span><span style='color:#037979;'>~<span style='color:black;font-weight:normal;'><br><br>  Change to a directory via abbreviations<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span><span style='color:#037979;'>d/s/9<span style='color:black;font-weight:normal;'><br><br>  Change to the previous working directory ($OLDPWD)<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'> </span></span></span><span style='color:#037979;'>-<span style='color:black;font-weight:normal;'><br><br></body></html></span></span>"
    );
    Ok(())
}

#[test]
#[ignore]
fn test_no_color_flag() -> Result {
    // TODO replace with something potentially more stable, otherwise this test needs to be
    // manually updated when ever the help output changes
    let code = "cd --help | to html --no-color";
    let outcome: String = test().run(code)?;
    assert_eq!(
        outcome,
        r"<html><style>body { background-color:white;color:black; }</style><body>Change directory.<br><br>Usage:<br>  &gt; cd (path) <br><br>Flags:<br>  -h, --help - Display the help message for this command<br><br>Parameters:<br>  path &lt;directory&gt;: The path to change to. (optional)<br><br>Input&#x2f;output types:<br>  ╭─#─┬──input──┬─output──╮<br>  │ 0 │ nothing │ nothing │<br>  │ 1 │ string  │ nothing │<br>  ╰───┴─────────┴─────────╯<br><br>Examples:<br>  Change to your home directory<br>  &gt; cd ~<br><br>  Change to the previous working directory ($OLDPWD)<br>  &gt; cd -<br><br></body></html>"
    );
    Ok(())
}

#[test]
fn test_list() -> Result {
    let code = "to html --list | where name == C64 | get 0";

    let outcome: HashMap<String, String> = test().run(code)?;
    assert_eq!(outcome["name"], "C64");
    assert_eq!(outcome["black"], "#090300");
    assert_eq!(outcome["red"], "#883932");
    assert_eq!(outcome["green"], "#55a049");
    assert_eq!(outcome["yellow"], "#bfce72");
    assert_eq!(outcome["blue"], "#40318d");
    assert_eq!(outcome["purple"], "#8b3f96");
    assert_eq!(outcome["cyan"], "#67b6bd");
    assert_eq!(outcome["white"], "#ffffff");
    assert_eq!(outcome["brightBlack"], "#000000");
    assert_eq!(outcome["brightRed"], "#883932");
    assert_eq!(outcome["brightGreen"], "#55a049");
    assert_eq!(outcome["brightYellow"], "#bfce72");
    assert_eq!(outcome["brightBlue"], "#40318d");
    assert_eq!(outcome["brightPurple"], "#8b3f96");
    assert_eq!(outcome["brightCyan"], "#67b6bd");
    assert_eq!(outcome["brightWhite"], "#f7f7f7");
    assert_eq!(outcome["background"], "#40318d");
    assert_eq!(outcome["foreground"], "#7869c4");
    assert_eq!(outcome.len(), 19);

    Ok(())
}
