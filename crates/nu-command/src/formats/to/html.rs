use crate::formats::to::delimited::merge_descriptors;
use fancy_regex::Regex;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned,
    SyntaxShape, Type, Value,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Write;

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

#[derive(Clone)]
pub struct ToHtml;

impl Command for ToHtml {
    fn name(&self) -> &str {
        "to html"
    }

    fn signature(&self) -> Signature {
        Signature::build("to html")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch("html-color", "change ansi colors to html colors", Some('c'))
            .switch("no-color", "remove all ansi colors in output", Some('n'))
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
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an  HTML string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to html",
                result: Some(Value::test_string(
                    r#"<html><style>body { background-color:white;color:black; }</style><body><table><thead><tr><th>foo</th><th>bar</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table></body></html>"#,
                )),
            },
            Example {
                description: "Optionally, only output the html for the content itself",
                example: "[[foo bar]; [1 2]] | to html --partial",
                result: Some(Value::test_string(
                    r#"<div style="background-color:white;color:black;"><table><thead><tr><th>foo</th><th>bar</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table></div>"#,
                )),
            },
            Example {
                description: "Optionally, output the string with a dark background",
                example: "[[foo bar]; [1 2]] | to html --dark",
                result: Some(Value::test_string(
                    r#"<html><style>body { background-color:black;color:white; }</style><body><table><thead><tr><th>foo</th><th>bar</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table></body></html>"#,
                )),
            },
        ]
    }

    fn usage(&self) -> &str {
        "Convert table into simple HTML"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, ShellError> {
        to_html(input, call, engine_state, stack)
    }
}

fn get_theme_from_asset_file(
    is_dark: bool,
    theme: &Option<Spanned<String>>,
) -> Result<HashMap<&'static str, String>, ShellError> {
    let theme_name = match theme {
        Some(s) => &s.item,
        None => "default", // There is no theme named "default" so this will be HtmlTheme::default(), which is "nu_default".
    };

    // 228 themes come from
    // https://github.com/mbadolato/iTerm2-Color-Schemes/tree/master/windowsterminal
    // we should find a hit on any name in there
    let asset = get_html_themes("228_themes.json").unwrap_or_default();

    // Find the theme by theme name
    let th = asset
        .themes
        .into_iter()
        .find(|n| n.name.to_lowercase() == theme_name.to_lowercase()) // case insensitive search
        .unwrap_or_default();

    Ok(convert_html_theme_to_hash_map(is_dark, &th))
}

fn convert_html_theme_to_hash_map(
    is_dark: bool,
    theme: &HtmlTheme,
) -> HashMap<&'static str, String> {
    let mut hm: HashMap<&str, String> = HashMap::with_capacity(18);

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

fn get_html_themes(json_name: &str) -> Result<HtmlThemes, Box<dyn Error>> {
    match Assets::get(json_name) {
        Some(content) => Ok(nu_json::from_slice(&content.data)?),
        None => Ok(HtmlThemes::default()),
    }
}

fn get_list_of_theme_names() -> Vec<String> {
    // If asset doesn't work, make sure to return the default theme
    let html_themes = get_html_themes("228_themes.json").unwrap_or_default();
    html_themes.themes.into_iter().map(|n| n.name).collect()
}

fn to_html(
    input: PipelineData,
    call: &Call,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let html_color = call.has_flag("html-color");
    let no_color = call.has_flag("no-color");
    let dark = call.has_flag("dark");
    let partial = call.has_flag("partial");
    let list = call.has_flag("list");
    let theme: Option<Spanned<String>> = call.get_flag(engine_state, stack, "theme")?;
    let config = engine_state.get_config();

    let vec_of_values = input.into_iter().collect::<Vec<Value>>();
    let headers = merge_descriptors(&vec_of_values);
    let headers = Some(headers)
        .filter(|headers| !headers.is_empty() && (headers.len() > 1 || !headers[0].is_empty()));
    let mut output_string = String::new();
    let mut regex_hm: HashMap<u32, (&str, String)> = HashMap::with_capacity(17);

    if list {
        // Get the list of theme names
        let theme_names = get_list_of_theme_names();

        // Put that list into the output string
        for s in &theme_names {
            writeln!(&mut output_string, "{}", s).unwrap();
        }

        output_string.push_str("\nScreenshots of themes can be found here:\n");
        output_string.push_str("https://github.com/mbadolato/iTerm2-Color-Schemes\n");
    } else {
        let theme_span = match &theme {
            Some(v) => v.span,
            None => head,
        };

        let color_hm = get_theme_from_asset_file(dark, &theme);
        let color_hm = match color_hm {
            Ok(c) => c,
            _ => {
                return Err(ShellError::GenericError(
                    "Error finding theme name".to_string(),
                    "Error finding theme name".to_string(),
                    Some(theme_span),
                    None,
                    Vec::new(),
                ))
            }
        };

        // change the color of the page
        if !partial {
            write!(
                &mut output_string,
                r"<html><style>body {{ background-color:{};color:{}; }}</style><body>",
                color_hm
                    .get("background")
                    .expect("Error getting background color"),
                color_hm
                    .get("foreground")
                    .expect("Error getting foreground color")
            )
            .unwrap();
        } else {
            write!(
                &mut output_string,
                "<div style=\"background-color:{};color:{};\">",
                color_hm
                    .get("background")
                    .expect("Error getting background color"),
                color_hm
                    .get("foreground")
                    .expect("Error getting foreground color")
            )
            .unwrap();
        }

        let inner_value = match vec_of_values.len() {
            0 => String::default(),
            1 => match headers {
                Some(headers) => html_table(vec_of_values, headers, config),
                None => {
                    let value = &vec_of_values[0];
                    html_value(value.clone(), config)
                }
            },
            _ => match headers {
                Some(headers) => html_table(vec_of_values, headers, config),
                None => html_list(vec_of_values, config),
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
    }
    Ok(Value::string(output_string, head).into_pipeline_data())
}

fn html_list(list: Vec<Value>, config: &Config) -> String {
    let mut output_string = String::new();
    output_string.push_str("<ol>");
    for value in list {
        output_string.push_str("<li>");
        output_string.push_str(&html_value(value, config));
        output_string.push_str("</li>");
    }
    output_string.push_str("</ol>");
    output_string
}

fn html_table(table: Vec<Value>, headers: Vec<String>, config: &Config) -> String {
    let mut output_string = String::new();

    output_string.push_str("<table>");

    output_string.push_str("<thead><tr>");
    for header in &headers {
        output_string.push_str("<th>");
        output_string.push_str(&htmlescape::encode_minimal(header));
        output_string.push_str("</th>");
    }
    output_string.push_str("</tr></thead><tbody>");

    for row in table {
        if let Value::Record { span, .. } = row {
            output_string.push_str("<tr>");
            for header in &headers {
                let data = row.get_data_by_key(header);
                output_string.push_str("<td>");
                output_string.push_str(&html_value(
                    data.unwrap_or_else(|| Value::nothing(span)),
                    config,
                ));
                output_string.push_str("</td>");
            }
            output_string.push_str("</tr>");
        }
    }
    output_string.push_str("</tbody></table>");

    output_string
}

fn html_value(value: Value, config: &Config) -> String {
    let mut output_string = String::new();
    match value {
        Value::Binary { val, .. } => {
            let output = nu_pretty_hex::pretty_hex(&val);
            output_string.push_str("<pre>");
            output_string.push_str(&output);
            output_string.push_str("</pre>");
        }
        other => output_string.push_str(
            &htmlescape::encode_minimal(&other.into_abbreviated_string(config))
                .replace('\n', "<br>"),
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
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToHtml {})
    }
}
