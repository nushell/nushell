use crate::prelude::*;
use futures::StreamExt;
use nu_data::value::format_leaf;
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::{AnchorLocation, Tagged};
use regex::Regex;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct HtmlThemes {
    themes: Vec<HtmlTheme>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
pub struct HtmlTheme {
    name: String,
    black: String,
    red: String,
    green: String,
    yellow: String,
    blue: String,
    purple: String,
    cyan: String,
    white: String,
    brightBlack: String,
    brightRed: String,
    brightGreen: String,
    brightYellow: String,
    brightBlue: String,
    brightPurple: String,
    brightCyan: String,
    brightWhite: String,
    background: String,
    foreground: String,
}

impl Default for HtmlThemes {
    fn default() -> Self {
        HtmlThemes {
            themes: vec![HtmlTheme::default()],
        }
    }
}

impl Default for HtmlTheme {
    fn default() -> Self {
        HtmlTheme {
            name: "nu_default".to_string(),
            black: "black".to_string(),
            red: "red".to_string(),
            green: "green".to_string(),
            yellow: "#717100".to_string(),
            blue: "blue".to_string(),
            purple: "#c800c8".to_string(),
            cyan: "#037979".to_string(),
            white: "white".to_string(),
            brightBlack: "black".to_string(),
            brightRed: "red".to_string(),
            brightGreen: "green".to_string(),
            brightYellow: "#717100".to_string(),
            brightBlue: "blue".to_string(),
            brightPurple: "#c800c8".to_string(),
            brightCyan: "#037979".to_string(),
            brightWhite: "white".to_string(),
            background: "white".to_string(),
            foreground: "black".to_string(),
        }
    }
}

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub struct ToHTML;

#[derive(Deserialize)]
pub struct ToHTMLArgs {
    html_color: bool,
    no_color: bool,
    dark: bool,
    partial: bool,
    theme: Option<Tagged<String>>,
    list: bool,
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
                "the name of the theme to use (github, blulocolight, ...)",
                Some('t'),
            )
            .switch("list", "list the names of all available themes", Some('l'))
    }

    fn usage(&self) -> &str {
        "Convert table into simple HTML"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        to_html(args).await
    }
}

fn get_theme_from_asset_file(
    is_dark: bool,
    theme: &Option<Tagged<String>>,
    theme_tag: &Tag,
) -> Result<HashMap<&'static str, String>, ShellError> {
    let theme_name = match theme {
        Some(s) => s.to_string(),
        None => "default".to_string(), // There is no theme named "default" so this will be HtmlTheme::default(), which is "nu_default".
    };

    // 228 themes come from
    // https://github.com/mbadolato/iTerm2-Color-Schemes/tree/master/windowsterminal
    // we should find a hit on any name in there
    let asset = get_asset_by_name_as_html_themes("228_themes.zip", "228_themes.json");

    // If asset doesn't work, make sure to return the default theme
    let asset = match asset {
        Ok(a) => a,
        _ => HtmlThemes::default(),
    };

    // Find the theme by theme name
    let th = asset
        .themes
        .iter()
        .find(|&n| n.name.to_lowercase() == *theme_name.to_lowercase().as_str()); // case insensitive search

    // If no theme is found by the name provided, ensure we return the default theme
    let default_theme = HtmlTheme::default();
    let th = match th {
        Some(t) => t,
        None => &default_theme,
    };

    // this just means no theme was passed in
    if th.name.to_lowercase().eq(&"nu_default".to_string())
        // this means there was a theme passed in
        && theme.is_some()
    {
        return Err(ShellError::labeled_error(
            "Error finding theme name",
            "Error finding theme name",
            theme_tag.span,
        ));
    }

    Ok(convert_html_theme_to_hash_map(is_dark, th))
}

#[allow(unused_variables)]
fn get_asset_by_name_as_html_themes(
    zip_name: &str,
    json_name: &str,
) -> Result<HtmlThemes, Box<dyn Error>> {
    match Assets::get(zip_name) {
        Some(content) => {
            let asset: Vec<u8> = match content {
                Cow::Borrowed(bytes) => bytes.into(),
                Cow::Owned(bytes) => bytes,
            };
            let reader = std::io::Cursor::new(asset);
            #[cfg(feature = "zip")]
            {
                use std::io::Read;
                let mut archive = zip::ZipArchive::new(reader)?;
                let mut zip_file = archive.by_name(json_name)?;
                let mut contents = String::new();
                zip_file.read_to_string(&mut contents)?;
                Ok(serde_json::from_str(&contents)?)
            }
            #[cfg(not(feature = "zip"))]
            {
                let th = HtmlThemes::default();
                Ok(th)
            }
        }
        None => {
            let th = HtmlThemes::default();
            Ok(th)
        }
    }
}

fn convert_html_theme_to_hash_map(
    is_dark: bool,
    theme: &HtmlTheme,
) -> HashMap<&'static str, String> {
    let mut hm: HashMap<&str, String> = HashMap::new();

    hm.insert("bold_black", theme.brightBlack[..].to_string());
    hm.insert("bold_red", theme.brightRed[..].to_string());
    hm.insert("bold_green", theme.brightGreen[..].to_string());
    hm.insert("bold_yellow", theme.brightYellow[..].to_string());
    hm.insert("bold_blue", theme.brightBlue[..].to_string());
    hm.insert("bold_magenta", theme.brightPurple[..].to_string());
    hm.insert("bold_cyan", theme.brightCyan[..].to_string());
    hm.insert("bold_white", theme.brightWhite[..].to_string());

    hm.insert("black", theme.black[..].to_string());
    hm.insert("red", theme.red[..].to_string());
    hm.insert("green", theme.green[..].to_string());
    hm.insert("yellow", theme.yellow[..].to_string());
    hm.insert("blue", theme.blue[..].to_string());
    hm.insert("magenta", theme.purple[..].to_string());
    hm.insert("cyan", theme.cyan[..].to_string());
    hm.insert("white", theme.white[..].to_string());

    // Try to make theme work with light or dark but
    // flipping the foreground and background but leave
    // the other colors the same.
    if is_dark {
        hm.insert("background", theme.black[..].to_string());
        hm.insert("foreground", theme.white[..].to_string());
    } else {
        hm.insert("background", theme.white[..].to_string());
        hm.insert("foreground", theme.black[..].to_string());
    }

    hm
}

fn get_list_of_theme_names() -> Vec<String> {
    let asset = get_asset_by_name_as_html_themes("228_themes.zip", "228_themes.json");

    // If asset doesn't work, make sure to return the default theme
    let html_themes = match asset {
        Ok(a) => a,
        _ => HtmlThemes::default(),
    };

    let theme_names: Vec<String> = html_themes
        .themes
        .iter()
        .map(|n| n.name[..].to_string())
        .collect();

    theme_names
}

async fn to_html(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let name_tag = args.call_info.name_tag.clone();
    let (
        ToHTMLArgs {
            html_color,
            no_color,
            dark,
            partial,
            theme,
            list,
        },
        input,
    ) = args.process().await?;
    let input: Vec<Value> = input.collect().await;
    let headers = nu_protocol::merge_descriptors(&input);
    let headers = Some(headers)
        .filter(|headers| !headers.is_empty() && (headers.len() > 1 || !headers[0].is_empty()));
    let mut output_string = String::new();
    let mut regex_hm: HashMap<u32, (&str, String)> = HashMap::new();

    if list {
        // Get the list of theme names
        let theme_names = get_list_of_theme_names();

        // Put that list into the output string
        for s in theme_names.iter() {
            output_string.push_str(&format!("{}\n", s));
        }

        output_string.push_str("\nScreenshots of themes can be found here:\n");
        output_string.push_str("https://github.com/mbadolato/iTerm2-Color-Schemes\n");

        // Short circuit and return the output_string
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(output_string).into_value(name_tag),
        )))
    } else {
        let theme_tag = match &theme {
            Some(v) => &v.tag,
            None => &name_tag,
        };

        let color_hm = get_theme_from_asset_file(dark, &theme, &theme_tag);
        let color_hm = match color_hm {
            Ok(c) => c,
            _ => {
                return Err(ShellError::labeled_error(
                    "Error finding theme name",
                    "Error finding theme name",
                    theme_tag.span,
                ))
            }
        };

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
            setup_html_color_regexes(&mut regex_hm, &color_hm);
            output_string = run_regexes(&regex_hm, &output_string);
        } else if no_color {
            setup_no_color_regexes(&mut regex_hm);
            output_string = run_regexes(&regex_hm, &output_string);
        }

        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(output_string).into_value(name_tag),
        )))
    }
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
    color_hm: &HashMap<&str, String>,
) {
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
    use super::ShellError;
    use super::*;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(ToHTML {})
    }
}
