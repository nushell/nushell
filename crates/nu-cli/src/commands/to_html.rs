use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use futures::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use nu_source::AnchorLocation;
use regex::Regex;
use std::collections::HashMap;

pub struct ToHTML;

#[derive(Deserialize)]
pub struct ToHTMLArgs {
    html_color: bool,
    no_color: bool,
    dark_bg: bool,
    use_campbell: bool,
}

#[async_trait]
impl WholeStreamCommand for ToHTML {
    fn name(&self) -> &str {
        "to html"
    }

    fn signature(&self) -> Signature {
        Signature::build("to html")
            .switch("html_color", "change ansi colors to html colors", Some('t'))
            .switch("no_color", "remove all ansi colors in output", Some('n'))
            .switch(
                "dark_bg",
                "indicate your background color is a darker color",
                Some('d'),
            )
            .switch(
                "use_campbell",
                "use microsoft's windows terminal color scheme named campell",
                Some('c'),
            )
    }

    fn usage(&self) -> &str {
        "Convert table into simple HTML"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_html(args, registry).await
    }
}

fn get_campbell_theme(is_dark: bool) -> HashMap<&'static str, String> {
    // for reference here is Microsoft's Campbell Theme
    // taken from here
    // https://docs.microsoft.com/en-us/windows/terminal/customize-settings/color-schemes
    let mut hm: HashMap<&str, String> = HashMap::new();

    if is_dark {
        hm.insert("bold_black", "#767676".to_string());
        hm.insert("bold_red", "#E74856".to_string());
        hm.insert("bold_green", "#16C60C".to_string());
        hm.insert("bold_yellow", "#F9F1A5".to_string());
        hm.insert("bold_blue", "#3B78FF".to_string());
        hm.insert("bold_magenta", "#B4009E".to_string());
        hm.insert("bold_cyan", "#61D6D6".to_string());
        hm.insert("bold_white", "#F2F2F2".to_string());

        hm.insert("black", "#0C0C0C".to_string());
        hm.insert("red", "#C50F1F".to_string());
        hm.insert("green", "#13A10E".to_string());
        hm.insert("yellow", "#C19C00".to_string());
        hm.insert("blue", "#0037DA".to_string());
        hm.insert("magenta", "#881798".to_string());
        hm.insert("cyan", "#3A96DD".to_string());
        hm.insert("white", "#CCCCCC".to_string());

        hm.insert("background", "#0C0C0C".to_string());
        hm.insert("foreground", "#CCCCCC".to_string());
    } else {
        hm.insert("bold_black", "#767676".to_string());
        hm.insert("bold_red", "#E74856".to_string());
        hm.insert("bold_green", "#16C60C".to_string());
        hm.insert("bold_yellow", "#F9F1A5".to_string());
        hm.insert("bold_blue", "#3B78FF".to_string());
        hm.insert("bold_magenta", "#B4009E".to_string());
        hm.insert("bold_cyan", "#61D6D6".to_string());
        hm.insert("bold_white", "#F2F2F2".to_string());

        hm.insert("black", "#0C0C0C".to_string());
        hm.insert("red", "#C50F1F".to_string());
        hm.insert("green", "#13A10E".to_string());
        hm.insert("yellow", "#C19C00".to_string());
        hm.insert("blue", "#0037DA".to_string());
        hm.insert("magenta", "#881798".to_string());
        hm.insert("cyan", "#3A96DD".to_string());
        hm.insert("white", "#CCCCCC".to_string());

        hm.insert("background", "#CCCCCC".to_string());
        hm.insert("foreground", "#0C0C0C".to_string());
    }

    hm
}

fn get_default_theme(is_dark: bool) -> HashMap<&'static str, String> {
    let mut hm: HashMap<&str, String> = HashMap::new();

    if is_dark {
        hm.insert("bold_black", "black".to_string());
        hm.insert("bold_red", "red".to_string());
        hm.insert("bold_green", "green".to_string());
        hm.insert("bold_yellow", "yellow".to_string());
        hm.insert("bold_blue", "blue".to_string());
        hm.insert("bold_magenta", "magenta".to_string());
        hm.insert("bold_cyan", "cyan".to_string());
        hm.insert("bold_white", "white".to_string());

        hm.insert("black", "black".to_string());
        hm.insert("red", "red".to_string());
        hm.insert("green", "green".to_string());
        hm.insert("yellow", "yellow".to_string());
        hm.insert("blue", "blue".to_string());
        hm.insert("magenta", "magenta".to_string());
        hm.insert("cyan", "cyan".to_string());
        hm.insert("white", "white".to_string());

        hm.insert("background", "black".to_string());
        hm.insert("foreground", "white".to_string());
    } else {
        hm.insert("bold_black", "black".to_string());
        hm.insert("bold_red", "red".to_string());
        hm.insert("bold_green", "green".to_string());
        hm.insert("bold_yellow", "#717100".to_string());
        hm.insert("bold_blue", "blue".to_string());
        hm.insert("bold_magenta", "#c800c8".to_string());
        hm.insert("bold_cyan", "#037979".to_string());
        hm.insert("bold_white", "white".to_string());

        hm.insert("black", "black".to_string());
        hm.insert("red", "red".to_string());
        hm.insert("green", "green".to_string());
        hm.insert("yellow", "#717100".to_string());
        hm.insert("blue", "blue".to_string());
        hm.insert("magenta", "#c800c8".to_string());
        hm.insert("cyan", "#037979".to_string());
        hm.insert("white", "white".to_string());

        hm.insert("background", "white".to_string());
        hm.insert("foreground", "black".to_string());
    }

    hm
}

fn get_colors(is_dark: bool, use_campbell: bool) -> HashMap<&'static str, String> {
    // Currently now using bold_white and bold_black.
    // This is not theming but it is kind of a start. The intent here is to use the
    // regular terminal colors which appear on black for most people and are very
    // high contrast. But when there's a light background, use something that works
    // better for it.

    if use_campbell {
        get_campbell_theme(is_dark)
    } else {
        get_default_theme(is_dark)
    }
}

async fn to_html(
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let name_tag = args.call_info.name_tag.clone();
    let (
        ToHTMLArgs {
            html_color,
            no_color,
            dark_bg,
            use_campbell,
        },
        input,
    ) = args.process(&registry).await?;
    let input: Vec<Value> = input.collect().await;
    let headers = nu_protocol::merge_descriptors(&input);
    let mut output_string = "<html>".to_string();
    let mut regex_hm: HashMap<u32, (&str, String)> = HashMap::new();
    let color_hm = get_colors(dark_bg, use_campbell);

    // change the color of the page
    output_string.push_str(&format!(
        r"<style>body {{ background-color:{};color:{}; }}</style><body>",
        color_hm
            .get("background")
            .expect("Error getting background color"),
        color_hm
            .get("foreground")
            .expect("Error getting foreground color")
    ));

    // Add grid lines to html
    // let mut output_string = "<html><head><style>".to_string();
    // output_string.push_str("table, th, td { border: 2px solid black; border-collapse: collapse; padding: 10px; }");
    // output_string.push_str("</style></head><body>");

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
        // output_string.push_str("<table>");

        // change the color of tables
        output_string.push_str(&format!(
            r"<table style='background-color:{};color:{};'>",
            color_hm
                .get("background")
                .expect("Error getting background color"),
            color_hm
                .get("foreground")
                .expect("Error getting foreground color")
        ));

        output_string.push_str("<tr>");

        for header in &headers {
            output_string.push_str("<th>");
            output_string.push_str(&htmlescape::encode_minimal(&header));
            output_string.push_str("</th>");
        }
        output_string.push_str("</tr>");
    }

    for row in input {
        match row.value {
            UntaggedValue::Primitive(Primitive::Binary(b)) => {
                // This might be a bit much, but it's fun :)
                match row.tag.anchor {
                    Some(AnchorLocation::Url(f)) | Some(AnchorLocation::File(f)) => {
                        let extension = f.split('.').last().map(String::from);
                        match extension {
                            Some(s)
                                if ["png", "jpg", "bmp", "gif", "tiff", "jpeg"]
                                    .contains(&s.to_lowercase().as_str()) =>
                            {
                                output_string.push_str("<img src=\"data:image/");
                                output_string.push_str(&s);
                                output_string.push_str(";base64,");
                                output_string.push_str(&base64::encode(&b));
                                output_string.push_str("\">");
                            }
                            _ => {
                                let output = pretty_hex::pretty_hex(&b);

                                output_string.push_str(&output);
                            }
                        }
                    }
                    _ => {
                        let output = pretty_hex::pretty_hex(&b);

                        output_string.push_str(&output);
                    }
                }
            }
            UntaggedValue::Primitive(Primitive::String(ref b)) => {
                // This might be a bit much, but it's fun :)
                match row.tag.anchor {
                    Some(AnchorLocation::Url(f)) | Some(AnchorLocation::File(f)) => {
                        let extension = f.split('.').last().map(String::from);
                        match extension {
                            Some(s) if s.to_lowercase() == "svg" => {
                                output_string.push_str("<img src=\"data:image/svg+xml;base64,");
                                output_string.push_str(&base64::encode(&b.as_bytes()));
                                output_string.push_str("\">");
                                continue;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                output_string.push_str(
                    &(htmlescape::encode_minimal(&format_leaf(&row.value).plain_string(100_000))
                        .replace("\n", "<br>")),
                );
            }
            UntaggedValue::Row(row) => {
                output_string.push_str("<tr>");
                for header in &headers {
                    let data = row.get_data(header);
                    output_string.push_str("<td>");
                    output_string.push_str(&format_leaf(data.borrow()).plain_string(100_000));
                    output_string.push_str("</td>");
                }
                output_string.push_str("</tr>");
            }
            p => {
                output_string.push_str(
                    &(htmlescape::encode_minimal(&format_leaf(&p).plain_string(100_000))
                        .replace("\n", "<br>")),
                );
            }
        }
    }

    if !headers.is_empty() && (headers.len() > 1 || headers[0] != "") {
        output_string.push_str("</table>");
    }
    output_string.push_str("</body></html>");

    // Check to see if we want to remove all color or change ansi to html colors
    if html_color {
        setup_html_color_regexes(&mut regex_hm, dark_bg, use_campbell);
        output_string = run_regexes(&regex_hm, &output_string);
    } else if no_color {
        setup_no_color_regexes(&mut regex_hm);
        output_string = run_regexes(&regex_hm, &output_string);
    }

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(output_string).into_value(name_tag),
    )))
}

fn setup_html_color_regexes(
    hash: &mut HashMap<u32, (&'static str, String)>,
    is_dark: bool,
    use_campbell: bool,
) {
    let color_hm = get_colors(is_dark, use_campbell);

    // All the bold colors
    hash.insert(
        0,
        (
            r"(?P<reset>\[0m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            // Reset the text color, normal weight font
            format!(
                r"<span style='color:{};font-weight:normal;'>$word</span>",
                color_hm
                    .get("foreground")
                    .expect("Error getting reset text color")
            ),
        ),
    );
    hash.insert(
        1,
        (
            // Bold Black
            r"(?P<bb>\[1;30m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("foreground")
                    .expect("Error getting bold black text color")
            ),
        ),
    );
    hash.insert(
        2,
        (
            // Bold Red
            r"(?P<br>\[1;31m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("bold_red")
                    .expect("Error getting bold red text color"),
            ),
        ),
    );
    hash.insert(
        3,
        (
            // Bold Green
            r"(?P<bg>\[1;32m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("bold_green")
                    .expect("Error getting bold green text color"),
            ),
        ),
    );
    hash.insert(
        4,
        (
            // Bold Yellow
            r"(?P<by>\[1;33m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("bold_yellow")
                    .expect("Error getting bold yellow text color"),
            ),
        ),
    );
    hash.insert(
        5,
        (
            // Bold Blue
            r"(?P<bu>\[1;34m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("bold_blue")
                    .expect("Error getting bold blue text color"),
            ),
        ),
    );
    hash.insert(
        6,
        (
            // Bold Magenta
            r"(?P<bm>\[1;35m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("bold_magenta")
                    .expect("Error getting bold magenta text color"),
            ),
        ),
    );
    hash.insert(
        7,
        (
            // Bold Cyan
            r"(?P<bc>\[1;36m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("bold_cyan")
                    .expect("Error getting bold cyan text color"),
            ),
        ),
    );
    hash.insert(
        8,
        (
            // Bold White
            // Let's change this to black since the html background
            // is white. White on white = no bueno.
            r"(?P<bw>\[1;37m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};font-weight:bold;'>$word</span>",
                color_hm
                    .get("foreground")
                    .expect("Error getting bold bold white text color"),
            ),
        ),
    );
    // All the normal colors
    hash.insert(
        9,
        (
            // Black
            r"(?P<b>\[30m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm
                    .get("foreground")
                    .expect("Error getting black text color"),
            ),
        ),
    );
    hash.insert(
        10,
        (
            // Red
            r"(?P<r>\[31m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm.get("red").expect("Error getting red text color"),
            ),
        ),
    );
    hash.insert(
        11,
        (
            // Green
            r"(?P<g>\[32m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm
                    .get("green")
                    .expect("Error getting green text color"),
            ),
        ),
    );
    hash.insert(
        12,
        (
            // Yellow
            r"(?P<y>\[33m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm
                    .get("yellow")
                    .expect("Error getting yellow text color"),
            ),
        ),
    );
    hash.insert(
        13,
        (
            // Blue
            r"(?P<u>\[34m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm.get("blue").expect("Error getting blue text color"),
            ),
        ),
    );
    hash.insert(
        14,
        (
            // Magenta
            r"(?P<m>\[35m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm
                    .get("magenta")
                    .expect("Error getting magenta text color"),
            ),
        ),
    );
    hash.insert(
        15,
        (
            // Cyan
            r"(?P<c>\[36m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm.get("cyan").expect("Error getting cyan text color"),
            ),
        ),
    );
    hash.insert(
        16,
        (
            // White
            // Let's change this to black since the html background
            // is white. White on white = no bueno.
            r"(?P<w>\[37m)(?P<word>[[:alnum:][:space:][:punct:]]*)",
            format!(
                r"<span style='color:{};'>$word</span>",
                color_hm
                    .get("foreground")
                    .expect("Error getting white text color"),
            ),
        ),
    );
}

fn setup_no_color_regexes(hash: &mut HashMap<u32, (&'static str, String)>) {
    // We can just use one regex here because we're just removing ansi sequences
    // and not replacing them with html colors.
    // attribution: https://stackoverflow.com/questions/14693701/how-can-i-remove-the-ansi-escape-sequences-from-a-string-in-python
    hash.insert(
        0,
        (
            r"(?:\x1B[@-Z\\-_]|[\x80-\x9A\x9C-\x9F]|(?:\x1B\[|\x9B)[0-?]*[ -/]*[@-~])",
            r"$name_group_doesnt_exist".to_string(),
        ),
    );
}

fn run_regexes(hash: &HashMap<u32, (&'static str, String)>, contents: &str) -> String {
    let mut working_string = contents.to_owned();
    let hash_count: u32 = hash.len() as u32;
    for n in 0..hash_count {
        let value = hash.get(&n).expect("error getting hash at index");
        //println!("{},{}", value.0, value.1);
        let re = Regex::new(value.0).expect("problem with color regex");
        let after = re.replace_all(&working_string, &value.1[..]).to_string();
        working_string = after.clone();
    }
    working_string
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ToHTML {})
    }

    #[test]
    fn test_cd_html_color_flag_dark_false() {
        let mut hm: HashMap<u32, (&str, String)> = HashMap::new();
        let cd_help = r"<html><style>body { background-color:white;color:black; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; [1;36mcd[0m[37m [0m[36mdirname[0m<br><br>  Change to your home directory<br>  &gt; [1;36mcd[0m<br><br>  Change to your home directory (alternate version)<br>  &gt; [1;36mcd[0m[37m [0m[36m~[0m<br><br>  Change to the previous directory<br>  &gt; [1;36mcd[0m[37m [0m[36m-[0m<br><br></body></html>".to_string();
        let cd_help_expected_result = r"<html><style>body { background-color:white;color:black; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; <span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#037979;'>dirname<span style='color:black;font-weight:normal;'><br><br>  Change to your home directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'><br><br>  Change to your home directory (alternate version)<br>  &gt; </span></span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#037979;'>~<span style='color:black;font-weight:normal;'><br><br>  Change to the previous directory<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>cd<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#037979;'>-<span style='color:black;font-weight:normal;'><br><br></body></html></span></span></span>".to_string();
        let is_dark = false;
        let use_campbell = false;
        setup_html_color_regexes(&mut hm, is_dark, use_campbell);
        assert_eq!(cd_help_expected_result, run_regexes(&hm, &cd_help));
    }

    #[test]
    fn test_cd_html_color_flag_dark_true() {
        let mut hm: HashMap<u32, (&str, String)> = HashMap::new();
        let cd_help = r"<html><style>body { background-color:black;color:white; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; [1;36mcd[0m[37m [0m[36mdirname[0m<br><br>  Change to your home directory<br>  &gt; [1;36mcd[0m<br><br>  Change to your home directory (alternate version)<br>  &gt; [1;36mcd[0m[37m [0m[36m~[0m<br><br>  Change to the previous directory<br>  &gt; [1;36mcd[0m[37m [0m[36m-[0m<br><br></body></html>".to_string();
        let cd_help_expected_result = r"<html><style>body { background-color:black;color:white; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; <span style='color:cyan;font-weight:bold;'>cd<span style='color:white;font-weight:normal;'></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:cyan;'>dirname<span style='color:white;font-weight:normal;'><br><br>  Change to your home directory<br>  &gt; </span><span style='color:cyan;font-weight:bold;'>cd<span style='color:white;font-weight:normal;'><br><br>  Change to your home directory (alternate version)<br>  &gt; </span></span><span style='color:cyan;font-weight:bold;'>cd<span style='color:white;font-weight:normal;'></span></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:cyan;'>~<span style='color:white;font-weight:normal;'><br><br>  Change to the previous directory<br>  &gt; </span><span style='color:cyan;font-weight:bold;'>cd<span style='color:white;font-weight:normal;'></span></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:cyan;'>-<span style='color:white;font-weight:normal;'><br><br></body></html></span></span></span>".to_string();
        let is_dark = true;
        let use_campbell = false;
        setup_html_color_regexes(&mut hm, is_dark, use_campbell);
        assert_eq!(cd_help_expected_result, run_regexes(&hm, &cd_help));
    }

    #[test]
    fn test_no_color_flag() {
        let mut hm: HashMap<u32, (&str, String)> = HashMap::new();
        let cd_help = r"<html><style>body { background-color:white;color:black; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; [1;36mcd[0m[37m [0m[36mdirname[0m<br><br>  Change to your home directory<br>  &gt; [1;36mcd[0m<br><br>  Change to your home directory (alternate version)<br>  &gt; [1;36mcd[0m[37m [0m[36m~[0m<br><br>  Change to the previous directory<br>  &gt; [1;36mcd[0m[37m [0m[36m-[0m<br><br></body></html>".to_string();
        let cd_help_expected_result = r"<html><style>body { background-color:white;color:black; }</style><body>Change to a new path.<br><br>Usage:<br>  &gt; cd (directory) {flags} <br><br>Parameters:<br>  (directory) the directory to change to<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  Change to a new directory called &#x27;dirname&#x27;<br>  &gt; cd dirname<br><br>  Change to your home directory<br>  &gt; cd<br><br>  Change to your home directory (alternate version)<br>  &gt; cd ~<br><br>  Change to the previous directory<br>  &gt; cd -<br><br></body></html>".to_string();
        setup_no_color_regexes(&mut hm);
        assert_eq!(cd_help_expected_result, run_regexes(&hm, &cd_help));
    }

    #[test]
    fn test_html_color_where_flag_dark_true() {
        let mut hm: HashMap<u32, (&str, String)> = HashMap::new();
        let where_help = r"<html><style>body { background-color:black;color:white; }</style><body>Filter table to match the condition.<br><br>Usage:<br>  &gt; where &lt;condition&gt; {flags} <br><br>Parameters:<br>  &lt;condition&gt; the condition that must match<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  List all files in the current directory with sizes greater than 2kb<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33msize[0m[37m [0m[33m&gt;[0m[37m [0m[1;35m2[0m[1;36mkb[0m<br><br>  List only the files in the current directory<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mtype[0m[37m [0m[33m==[0m[37m [0m[32mFile[0m<br><br>  List all files with names that contain &quot;Car&quot;<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mname[0m[37m [0m[33m=~[0m[37m [0m[32m&quot;Car&quot;[0m<br><br>  List all files that were modified in the last two months<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mmodified[0m[37m [0m[33m&lt;=[0m[37m [0m[1;35m2[0m[1;36mM[0m<br><br></body></html>".to_string();
        let where_help_exptected_results = r"<html><style>body { background-color:black;color:white; }</style><body>Filter table to match the condition.<br><br>Usage:<br>  &gt; where &lt;condition&gt; {flags} <br><br>Parameters:<br>  &lt;condition&gt; the condition that must match<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  List all files in the current directory with sizes greater than 2kb<br>  &gt; <span style='color:cyan;font-weight:bold;'>ls<span style='color:white;font-weight:normal;'></span></span><span style='color:white;'> | <span style='color:white;font-weight:normal;'></span><span style='color:cyan;font-weight:bold;'>where<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;font-weight:bold;'>size<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;'>&gt;<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:magenta;font-weight:bold;'>2<span style='color:white;font-weight:normal;'></span></span><span style='color:cyan;font-weight:bold;'>kb<span style='color:white;font-weight:normal;'><br><br>  List only the files in the current directory<br>  &gt; </span></span><span style='color:cyan;font-weight:bold;'>ls<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> | <span style='color:white;font-weight:normal;'></span><span style='color:cyan;font-weight:bold;'>where<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;font-weight:bold;'>type<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;'>==<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:green;'>File<span style='color:white;font-weight:normal;'><br><br>  List all files with names that contain &quot;Car&quot;<br>  &gt; </span><span style='color:cyan;font-weight:bold;'>ls<span style='color:white;font-weight:normal;'></span></span></span></span><span style='color:white;'> | <span style='color:white;font-weight:normal;'></span><span style='color:cyan;font-weight:bold;'>where<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;font-weight:bold;'>name<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;'>=~<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:green;'>&quot;Car&quot;<span style='color:white;font-weight:normal;'><br><br>  List all files that were modified in the last two months<br>  &gt; </span><span style='color:cyan;font-weight:bold;'>ls<span style='color:white;font-weight:normal;'></span></span></span></span><span style='color:white;'> | <span style='color:white;font-weight:normal;'></span><span style='color:cyan;font-weight:bold;'>where<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;font-weight:bold;'>modified<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:yellow;'>&lt;=<span style='color:white;font-weight:normal;'></span></span></span><span style='color:white;'> <span style='color:white;font-weight:normal;'></span><span style='color:magenta;font-weight:bold;'>2<span style='color:white;font-weight:normal;'></span></span><span style='color:cyan;font-weight:bold;'>M<span style='color:white;font-weight:normal;'><br><br></body></html></span></span></span>".to_string();
        let is_dark = true;
        let use_campbell = false;
        setup_html_color_regexes(&mut hm, is_dark, use_campbell);
        assert_eq!(where_help_exptected_results, run_regexes(&hm, &where_help));
    }

    #[test]
    fn test_html_color_where_flag_dark_false() {
        let mut hm: HashMap<u32, (&str, String)> = HashMap::new();
        let where_help = r"<html><style>body { background-color:white;color:black; }</style><body>Filter table to match the condition.<br><br>Usage:<br>  &gt; where &lt;condition&gt; {flags} <br><br>Parameters:<br>  &lt;condition&gt; the condition that must match<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  List all files in the current directory with sizes greater than 2kb<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33msize[0m[37m [0m[33m&gt;[0m[37m [0m[1;35m2[0m[1;36mkb[0m<br><br>  List only the files in the current directory<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mtype[0m[37m [0m[33m==[0m[37m [0m[32mFile[0m<br><br>  List all files with names that contain &quot;Car&quot;<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mname[0m[37m [0m[33m=~[0m[37m [0m[32m&quot;Car&quot;[0m<br><br>  List all files that were modified in the last two months<br>  &gt; [1;36mls[0m[37m | [0m[1;36mwhere[0m[37m [0m[1;33mmodified[0m[37m [0m[33m&lt;=[0m[37m [0m[1;35m2[0m[1;36mM[0m<br><br></body></html>".to_string();
        let where_help_exptected_results = r"<html><style>body { background-color:white;color:black; }</style><body>Filter table to match the condition.<br><br>Usage:<br>  &gt; where &lt;condition&gt; {flags} <br><br>Parameters:<br>  &lt;condition&gt; the condition that must match<br><br>Flags:<br>  -h, --help: Display this help message<br><br>Examples:<br>  List all files in the current directory with sizes greater than 2kb<br>  &gt; <span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>size<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>&gt;<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#c800c8;font-weight:bold;'>2<span style='color:black;font-weight:normal;'></span></span><span style='color:#037979;font-weight:bold;'>kb<span style='color:black;font-weight:normal;'><br><br>  List only the files in the current directory<br>  &gt; </span></span><span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>type<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>==<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:green;'>File<span style='color:black;font-weight:normal;'><br><br>  List all files with names that contain &quot;Car&quot;<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>name<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>=~<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:green;'>&quot;Car&quot;<span style='color:black;font-weight:normal;'><br><br>  List all files that were modified in the last two months<br>  &gt; </span><span style='color:#037979;font-weight:bold;'>ls<span style='color:black;font-weight:normal;'></span></span></span></span><span style='color:black;'> | <span style='color:black;font-weight:normal;'></span><span style='color:#037979;font-weight:bold;'>where<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;font-weight:bold;'>modified<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#717100;'>&lt;=<span style='color:black;font-weight:normal;'></span></span></span><span style='color:black;'> <span style='color:black;font-weight:normal;'></span><span style='color:#c800c8;font-weight:bold;'>2<span style='color:black;font-weight:normal;'></span></span><span style='color:#037979;font-weight:bold;'>M<span style='color:black;font-weight:normal;'><br><br></body></html></span></span></span>".to_string();
        let is_dark = false;
        let use_campbell = false;
        setup_html_color_regexes(&mut hm, is_dark, use_campbell);
        assert_eq!(where_help_exptected_results, run_regexes(&hm, &where_help));
    }
}
