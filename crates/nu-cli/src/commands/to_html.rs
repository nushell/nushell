use crate::commands::WholeStreamCommand;
use crate::data::value::format_leaf;
use crate::prelude::*;
use futures::StreamExt;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tagged};
use regex::Regex;
use std::collections::HashMap;

pub struct ToHTML;

#[derive(Deserialize)]
pub struct ToHTMLArgs {
    html_color: bool,
    no_color: bool,
    dark: bool,
    partial: bool,
    theme: Option<Tagged<String>>,
}

#[async_trait]
impl WholeStreamCommand for ToHTML {
    fn name(&self) -> &str {
        "to html"
    }

    fn signature(&self) -> Signature {
        Signature::build("to html")
            .switch("html_color", "change ansi colors to html colors", Some('c'))
            .switch("no_color", "remove all ansi colors in output", Some('n'))
            .switch(
                "dark",
                "indicate your background color is a darker color",
                Some('d'),
            )
            .switch(
                "partial",
                "only output the html for the content itself",
                Some('p'),
            )
            .named(
                "theme",
                SyntaxShape::String,
                "the name of the theme to use (default, campbell, github, blulocolight)",
                Some('t'),
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

    // Try to make theme work with light or dark but
    // flipping the foreground and background but leave
    // the other colors the same.
    if is_dark {
        hm.insert("background", "#0C0C0C".to_string());
        hm.insert("foreground", "#CCCCCC".to_string());
    } else {
        hm.insert("background", "#CCCCCC".to_string());
        hm.insert("foreground", "#0C0C0C".to_string());
    }

    hm
}

fn get_default_theme(is_dark: bool) -> HashMap<&'static str, String> {
    let mut hm: HashMap<&str, String> = HashMap::new();

    // This theme has different colors for dark and light
    // so we can't just swap the background colors.
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

fn get_github_theme(is_dark: bool) -> HashMap<&'static str, String> {
    // Suggested by Jörn for use with demo site
    // Taken from here https://github.com/mbadolato/iTerm2-Color-Schemes/blob/master/windowsterminal/Github.json
    // This is a light theme named github, intended for a white background
    // The next step will be to load these json themes if we ever get to that point
    let mut hm: HashMap<&str, String> = HashMap::new();

    hm.insert("bold_black", "#666666".to_string());
    hm.insert("bold_red", "#de0000".to_string());
    hm.insert("bold_green", "#87d5a2".to_string());
    hm.insert("bold_yellow", "#f1d007".to_string());
    hm.insert("bold_blue", "#2e6cba".to_string());
    hm.insert("bold_magenta", "#ffa29f".to_string());
    hm.insert("bold_cyan", "#1cfafe".to_string());
    hm.insert("bold_white", "#ffffff".to_string());

    hm.insert("black", "#3e3e3e".to_string());
    hm.insert("red", "#970b16".to_string());
    hm.insert("green", "#07962a".to_string());
    hm.insert("yellow", "#f8eec7".to_string());
    hm.insert("blue", "#003e8a".to_string());
    hm.insert("magenta", "#e94691".to_string());
    hm.insert("cyan", "#89d1ec".to_string());
    hm.insert("white", "#ffffff".to_string());

    // Try to make theme work with light or dark but
    // flipping the foreground and background but leave
    // the other colors the same.
    if is_dark {
        hm.insert("background", "#3e3e3e".to_string());
        hm.insert("foreground", "#f4f4f4".to_string());
    } else {
        hm.insert("background", "#f4f4f4".to_string());
        hm.insert("foreground", "#3e3e3e".to_string());
    }

    hm
}

fn get_blulocolight_theme(is_dark: bool) -> HashMap<&'static str, String> {
    let mut hm: HashMap<&str, String> = HashMap::new();

    hm.insert("bold_black", "#dedfe8".to_string());
    hm.insert("bold_red", "#fc4a6d".to_string());
    hm.insert("bold_green", "#34b354".to_string());
    hm.insert("bold_yellow", "#b89427".to_string());
    hm.insert("bold_blue", "#1085d9".to_string());
    hm.insert("bold_magenta", "#c00db3".to_string());
    hm.insert("bold_cyan", "#5b80ad".to_string());
    hm.insert("bold_white", "#1d1d22".to_string());

    hm.insert("black", "#cbccd5".to_string());
    hm.insert("red", "#c90e42".to_string());
    hm.insert("green", "#21883a".to_string());
    hm.insert("yellow", "#d54d17".to_string());
    hm.insert("blue", "#1e44dd".to_string());
    hm.insert("magenta", "#6d1bed".to_string());
    hm.insert("cyan", "#1f4d7a".to_string());
    hm.insert("white", "#000000".to_string());

    // Try to make theme work with light or dark but
    // flipping the foreground and background but leave
    // the other colors the same.
    if is_dark {
        hm.insert("background", "#2a2c33".to_string());
        hm.insert("foreground", "#f7f7f7".to_string());
    } else {
        hm.insert("background", "#f7f7f7".to_string());
        hm.insert("foreground", "#2a2c33".to_string());
    }

    hm
}

fn get_colors(is_dark: bool, theme: &Option<Tagged<String>>) -> HashMap<&'static str, String> {
    let theme_name = match theme {
        Some(s) => s.to_string(),
        None => "default".to_string(),
    };

    match theme_name.as_ref() {
        "default" => get_default_theme(is_dark),
        "campbell" => get_campbell_theme(is_dark),
        "github" => get_github_theme(is_dark),
        "blulocolight" => get_blulocolight_theme(is_dark),
        _ => get_default_theme(is_dark),
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
            dark,
            partial,
            theme,
        },
        input,
    ) = args.process(&registry).await?;
    let input: Vec<Value> = input.collect().await;
    let headers = nu_protocol::merge_descriptors(&input);
    let headers = Some(headers)
        .filter(|headers| !headers.is_empty() && (headers.len() > 1 || headers[0] != ""));
    let mut output_string = String::new();
    let mut regex_hm: HashMap<u32, (&str, String)> = HashMap::new();
    let color_hm = get_colors(dark, &theme);

    // change the color of the page
    if !partial {
        output_string.push_str(&format!(
            r"<html><style>body {{ background-color:{};color:{}; }}</style><body>",
            color_hm
                .get("background")
                .expect("Error getting background color"),
            color_hm
                .get("foreground")
                .expect("Error getting foreground color")
        ));
    } else {
        output_string.push_str(&format!(
            "<div style=\"background-color:{};color:{};\">",
            color_hm
                .get("background")
                .expect("Error getting background color"),
            color_hm
                .get("foreground")
                .expect("Error getting foreground color")
        ));
    }

    let inner_value = match input.len() {
        0 => String::default(),
        1 => match headers {
            Some(headers) => html_table(input, headers),
            None => {
                let value = &input[0];
                html_value(value)
            }
        },
        _ => match headers {
            Some(headers) => html_table(input, headers),
            None => html_list(input),
        },
    };

    output_string.push_str(&inner_value);

    if !partial {
        output_string.push_str("</body></html>");
    } else {
        output_string.push_str("</div>")
    }

    // Check to see if we want to remove all color or change ansi to html colors
    if html_color {
        setup_html_color_regexes(&mut regex_hm, dark, &theme);
        output_string = run_regexes(&regex_hm, &output_string);
    } else if no_color {
        setup_no_color_regexes(&mut regex_hm);
        output_string = run_regexes(&regex_hm, &output_string);
    }

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(output_string).into_value(name_tag),
    )))
}

fn html_list(list: Vec<Value>) -> String {
    let mut output_string = String::new();
    output_string.push_str("<ol>");
    for value in list {
        output_string.push_str("<li>");
        output_string.push_str(&html_value(&value));
        output_string.push_str("</li>");
    }
    output_string.push_str("</ol>");
    output_string
}

fn html_table(table: Vec<Value>, headers: Vec<String>) -> String {
    let mut output_string = String::new();
    // Add grid lines to html
    // let mut output_string = "<html><head><style>".to_string();
    // output_string.push_str("table, th, td { border: 2px solid black; border-collapse: collapse; padding: 10px; }");
    // output_string.push_str("</style></head><body>");

    output_string.push_str("<table>");

    output_string.push_str("<tr>");
    for header in &headers {
        output_string.push_str("<th>");
        output_string.push_str(&htmlescape::encode_minimal(&header));
        output_string.push_str("</th>");
    }
    output_string.push_str("</tr>");

    for row in table {
        if let UntaggedValue::Row(row) = row.value {
            output_string.push_str("<tr>");
            for header in &headers {
                let data = row.get_data(header);
                output_string.push_str("<td>");
                output_string.push_str(&html_value(data.borrow()));
                output_string.push_str("</td>");
            }
            output_string.push_str("</tr>");
        }
    }
    output_string.push_str("</table>");

    output_string
}

fn html_value(value: &Value) -> String {
    let mut output_string = String::new();
    match &value.value {
        UntaggedValue::Primitive(Primitive::Binary(b)) => {
            // This might be a bit much, but it's fun :)
            match &value.tag.anchor {
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

                            output_string.push_str("<pre>");
                            output_string.push_str(&output);
                            output_string.push_str("</pre>");
                        }
                    }
                }
                _ => {
                    let output = pretty_hex::pretty_hex(&b);

                    output_string.push_str("<pre>");
                    output_string.push_str(&output);
                    output_string.push_str("</pre>");
                }
            }
        }
        UntaggedValue::Primitive(Primitive::String(ref b)) => {
            // This might be a bit much, but it's fun :)
            match &value.tag.anchor {
                Some(AnchorLocation::Url(f)) | Some(AnchorLocation::File(f)) => {
                    let extension = f.split('.').last().map(String::from);
                    match extension {
                        Some(s) if s.to_lowercase() == "svg" => {
                            output_string.push_str("<img src=\"data:image/svg+xml;base64,");
                            output_string.push_str(&base64::encode(&b.as_bytes()));
                            output_string.push_str("\">");
                            return output_string;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            output_string.push_str(
                &htmlescape::encode_minimal(&format_leaf(&value.value).plain_string(100_000))
                    .replace("\n", "<br>"),
            );
        }
        other => output_string.push_str(
            &htmlescape::encode_minimal(&format_leaf(other).plain_string(100_000))
                .replace("\n", "<br>"),
        ),
    }
    output_string
}

fn setup_html_color_regexes(
    hash: &mut HashMap<u32, (&'static str, String)>,
    is_dark: bool,
    theme: &Option<Tagged<String>>,
) {
    let color_hm = get_colors(is_dark, theme);

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

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ToHTML {})
    }
}
