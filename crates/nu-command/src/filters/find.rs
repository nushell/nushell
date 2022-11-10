use crate::help::highlight_search_string;

use fancy_regex::Regex;
use lscolors::{Color as LsColors_Color, LsColors, Style as LsColors_Style};
use nu_ansi_term::{Color, Color::Default, Style};
use nu_color_config::get_color_config;
use nu_engine::{env_to_string, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Config, Example, IntoInterruptiblePipelineData, ListStream, PipelineData, ShellError,
    Signature, Span, SyntaxShape, Type, Value,
};
use nu_utils::get_ls_colors;

#[derive(Clone)]
pub struct Find;

impl Command for Find {
    fn name(&self) -> &str {
        "find"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    // TODO: This is too permissive; if we could express this
                    // using a type parameter it would be List<T> -> List<T>.
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::String, Type::String),
                (
                    // For find -p
                    Type::Table(vec![]),
                    Type::Table(vec![]),
                ),
            ])
            .named(
                "regex",
                SyntaxShape::String,
                "regex to match with",
                Some('r'),
            )
            .switch(
                "insensitive",
                "case-insensitive search for regex (?i)",
                Some('i'),
            )
            .switch(
                "multiline",
                "multi-line mode: ^ and $ match begin/end of line for regex (?m)",
                Some('m'),
            )
            .switch(
                "dotall",
                "dotall mode: allow a dot . to match newline character \\n for regex (?s)",
                Some('s'),
            )
            .switch("invert", "invert the match", Some('v'))
            .rest("rest", SyntaxShape::Any, "terms to search")
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Searches terms in the input."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Search for multiple terms in a command output",
                example: r#"ls | find toml md sh"#,
                result: None,
            },
            Example {
                description: "Search for a term in a string",
                example: r#"'Cargo.toml' | find toml"#,
                result: Some(Value::test_string("Cargo.toml".to_owned())),
            },
            Example {
                description: "Search a number or a file size in a list of numbers",
                example: r#"[1 5 3kb 4 3Mb] | find 5 3kb"#,
                result: Some(Value::List {
                    vals: vec![Value::test_int(5), Value::test_filesize(3000)],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Search a char in a list of string",
                example: r#"[moe larry curly] | find l"#,
                result: Some(Value::List {
                    vals: vec![Value::test_string("larry"), Value::test_string("curly")],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find using regex",
                example: r#"[abc bde arc abf] | find --regex "ab""#,
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("abc".to_string()),
                        Value::test_string("abf".to_string()),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find using regex case insensitive",
                example: r#"[aBc bde Arc abf] | find --regex "ab" -i"#,
                result: Some(Value::List {
                    vals: vec![
                        Value::test_string("aBc".to_string()),
                        Value::test_string("abf".to_string()),
                    ],
                    span: Span::test_data(),
                }),
            },
            Example {
                description: "Find value in records",
                example: r#"[[version name]; [0.1.0 nushell] [0.1.1 fish] [0.2.0 zsh]] | find -r "nu""#,
                result: Some(Value::List {
                    vals: vec![Value::test_record(
                        vec!["version", "name"],
                        vec![
                            Value::test_string("0.1.0"),
                            Value::test_string("nushell".to_string()),
                        ],
                    )],
                    span: Span::test_data(),
                }),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["filter", "regex", "search", "condition"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let regex = call.get_flag::<String>(engine_state, stack, "regex")?;

        if let Some(regex) = regex {
            find_with_regex(regex, engine_state, stack, call, input)
        } else {
            find_with_rest_and_highlight(engine_state, stack, call, input)
        }
    }
}

fn find_with_regex(
    regex: String,
    engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrlc = engine_state.ctrlc.clone();
    let config = engine_state.get_config().clone();

    let insensitive = call.has_flag("insensitive");
    let multiline = call.has_flag("multiline");
    let dotall = call.has_flag("dotall");
    let invert = call.has_flag("invert");

    let flags = match (insensitive, multiline, dotall) {
        (false, false, false) => "",
        (true, false, false) => "(?i)",
        (false, true, false) => "(?m)",
        (false, false, true) => "(?s)",
        (true, true, false) => "(?im)",
        (true, false, true) => "(?is)",
        (false, true, true) => "(?ms)",
        (true, true, true) => "(?ims)",
    };

    let regex = flags.to_string() + regex.as_str();

    let re = Regex::new(regex.as_str())
        .map_err(|e| ShellError::UnsupportedInput(format!("incorrect regex: {}", e), span))?;

    input.filter(
        move |value| match value {
            Value::String { val, .. } => re.is_match(val.as_str()).unwrap_or(false) != invert,
            Value::Record { cols: _, vals, .. } => {
                let matches: Vec<bool> = vals
                    .iter()
                    .map(|v| {
                        re.is_match(v.into_string(" ", &config).as_str())
                            .unwrap_or(false)
                            != invert
                    })
                    .collect();
                matches.iter().any(|b| *b)
            }
            Value::List { vals, .. } => {
                let matches: Vec<bool> = vals
                    .iter()
                    .map(|v| {
                        re.is_match(v.into_string(" ", &config).as_str())
                            .unwrap_or(false)
                            != invert
                    })
                    .collect();
                matches.iter().any(|b| *b)
            }
            _ => false,
        },
        ctrlc,
    )
}

fn highlight_terms_in_record(
    cols: &mut [String],
    vals: &mut Vec<Value>,
    span: &mut Span,
    config: &Config,
    terms: &[Value],
    string_style: Style,
    ls_colors: &LsColors,
) -> Value {
    let mut output = vec![];
    for val in vals {
        let val_str = val.into_string("", config);
        let lower_val = val.into_string("", config).to_lowercase();
        let mut term_added_to_output = false;
        for term in terms {
            let term_str = term.into_string("", config);
            let lower_term = term.into_string("", config).to_lowercase();
            if lower_val.contains(&lower_term) {
                if config.use_ls_colors {
                    // Get the original LS_COLORS color
                    let style = ls_colors.style_for_path(val_str.clone());
                    let ansi_style = style
                        .map(LsColors_Style::to_crossterm_style)
                        .unwrap_or_default();

                    let ls_colored_val = ansi_style.apply(&val_str).to_string();

                    let ansi_term_style = style
                        .map(to_nu_ansi_term_style)
                        .unwrap_or_else(|| string_style);

                    let hi =
                        match highlight_search_string(&ls_colored_val, &term_str, &ansi_term_style)
                        {
                            Ok(hi) => hi,
                            Err(_) => string_style.paint(term_str.to_string()).to_string(),
                        };
                    output.push(Value::String {
                        val: hi,
                        span: *span,
                    });
                    term_added_to_output = true;
                } else {
                    // No LS_COLORS support, so just use the original value
                    let hi = match highlight_search_string(&val_str, &term_str, &string_style) {
                        Ok(hi) => hi,
                        Err(_) => string_style.paint(term_str.to_string()).to_string(),
                    };
                    output.push(Value::String {
                        val: hi,
                        span: *span,
                    });
                }
            }
        }
        if !term_added_to_output {
            output.push(val.clone());
        }
    }
    Value::Record {
        cols: cols.to_vec(),
        vals: output,
        span: *span,
    }
}

fn find_with_rest_and_highlight(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrlc = engine_state.ctrlc.clone();
    let engine_state = engine_state.clone();
    let config = engine_state.get_config().clone();
    let filter_config = engine_state.get_config().clone();
    let invert = call.has_flag("invert");
    let terms = call.rest::<Value>(&engine_state, stack, 0)?;
    let lower_terms = terms
        .iter()
        .map(|v| {
            if let Ok(span) = v.span() {
                Value::string(v.into_string("", &config).to_lowercase(), span)
            } else {
                v.clone()
            }
        })
        .collect::<Vec<Value>>();

    let color_hm = get_color_config(&config);
    let default_style = Style::new().fg(Default).on(Default);
    let string_style = match color_hm.get("string") {
        Some(style) => *style,
        None => default_style,
    };
    let ls_colors_env_str = match stack.get_env_var(&engine_state, "LS_COLORS") {
        Some(v) => Some(env_to_string("LS_COLORS", &v, &engine_state, stack)?),
        None => None,
    };
    let ls_colors = get_ls_colors(ls_colors_env_str);

    match input {
        PipelineData::Value(_, _) => input
            .map(
                move |mut x| match &mut x {
                    Value::Record { cols, vals, span } => highlight_terms_in_record(
                        cols,
                        vals,
                        span,
                        &config,
                        &terms,
                        string_style,
                        &ls_colors,
                    ),
                    _ => x,
                },
                ctrlc.clone(),
            )?
            .filter(
                move |value| {
                    let lower_value = if let Ok(span) = value.span() {
                        Value::string(value.into_string("", &filter_config).to_lowercase(), span)
                    } else {
                        value.clone()
                    };

                    lower_terms.iter().any(|term| match value {
                        Value::Bool { .. }
                        | Value::Int { .. }
                        | Value::Filesize { .. }
                        | Value::Duration { .. }
                        | Value::Date { .. }
                        | Value::Range { .. }
                        | Value::Float { .. }
                        | Value::Block { .. }
                        | Value::Closure { .. }
                        | Value::Nothing { .. }
                        | Value::Error { .. } => lower_value
                            .eq(span, term, span)
                            .map_or(false, |val| val.is_true()),
                        Value::String { .. }
                        | Value::List { .. }
                        | Value::CellPath { .. }
                        | Value::CustomValue { .. } => term
                            .r#in(span, &lower_value, span)
                            .map_or(false, |val| val.is_true()),
                        Value::Record { vals, .. } => vals.iter().any(|val| {
                            if let Ok(span) = val.span() {
                                let lower_val = Value::string(
                                    val.into_string("", &filter_config).to_lowercase(),
                                    Span::test_data(),
                                );

                                term.r#in(span, &lower_val, span)
                                    .map_or(false, |aval| aval.is_true())
                            } else {
                                term.r#in(span, val, span)
                                    .map_or(false, |aval| aval.is_true())
                            }
                        }),
                        Value::Binary { .. } => false,
                    }) != invert
                },
                ctrlc,
            ),
        PipelineData::ListStream(stream, meta) => Ok(ListStream::from_stream(
            stream
                .map(move |mut x| match &mut x {
                    Value::Record { cols, vals, span } => highlight_terms_in_record(
                        cols,
                        vals,
                        span,
                        &config,
                        &terms,
                        string_style,
                        &ls_colors,
                    ),
                    _ => x,
                })
                .filter(move |value| {
                    let lower_value = if let Ok(span) = value.span() {
                        Value::string(value.into_string("", &filter_config).to_lowercase(), span)
                    } else {
                        value.clone()
                    };

                    lower_terms.iter().any(|term| match value {
                        Value::Bool { .. }
                        | Value::Int { .. }
                        | Value::Filesize { .. }
                        | Value::Duration { .. }
                        | Value::Date { .. }
                        | Value::Range { .. }
                        | Value::Float { .. }
                        | Value::Block { .. }
                        | Value::Closure { .. }
                        | Value::Nothing { .. }
                        | Value::Error { .. } => lower_value
                            .eq(span, term, span)
                            .map_or(false, |value| value.is_true()),
                        Value::String { .. }
                        | Value::List { .. }
                        | Value::CellPath { .. }
                        | Value::CustomValue { .. } => term
                            .r#in(span, &lower_value, span)
                            .map_or(false, |value| value.is_true()),
                        Value::Record { vals, .. } => vals.iter().any(|val| {
                            if let Ok(span) = val.span() {
                                let lower_val = Value::string(
                                    val.into_string("", &filter_config).to_lowercase(),
                                    Span::test_data(),
                                );

                                term.r#in(span, &lower_val, span)
                                    .map_or(false, |value| value.is_true())
                            } else {
                                term.r#in(span, val, span)
                                    .map_or(false, |value| value.is_true())
                            }
                        }),
                        Value::Binary { .. } => false,
                    }) != invert
                }),
            ctrlc.clone(),
        )
        .into_pipeline_data(ctrlc)
        .set_metadata(meta)),
        PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(span)),
        PipelineData::ExternalStream {
            stdout: Some(stream),
            ..
        } => {
            let mut output: Vec<Value> = vec![];
            for filter_val in stream {
                match filter_val {
                    Ok(value) => match value {
                        Value::String { val, span } => {
                            let split_char = if val.contains("\r\n") { "\r\n" } else { "\n" };

                            for line in val.split(split_char) {
                                for term in lower_terms.iter() {
                                    let term_str = term.into_string("", &filter_config);
                                    let lower_val = line.to_lowercase();
                                    if lower_val
                                        .contains(&term.into_string("", &config).to_lowercase())
                                    {
                                        output.push(Value::String {
                                            val: highlight_search_string(
                                                line,
                                                &term_str,
                                                &string_style,
                                            )?,
                                            span,
                                        })
                                    }
                                }
                            }
                        }
                        _ => {
                            return Err(ShellError::UnsupportedInput(
                                format!(
                                    "Unsupport value type '{}' from raw stream",
                                    value.get_type()
                                ),
                                span,
                            ))
                        }
                    },
                    _ => {
                        return Err(ShellError::UnsupportedInput(
                            "Unsupport type from raw stream".to_string(),
                            span,
                        ))
                    }
                };
            }
            Ok(output.into_pipeline_data(ctrlc))
        }
    }
}

fn to_nu_ansi_term_style(style: &LsColors_Style) -> Style {
    fn to_nu_ansi_term_color(color: &LsColors_Color) -> Color {
        match *color {
            LsColors_Color::Fixed(n) => Color::Fixed(n),
            LsColors_Color::RGB(r, g, b) => Color::Rgb(r, g, b),
            LsColors_Color::Black => Color::Black,
            LsColors_Color::Red => Color::Red,
            LsColors_Color::Green => Color::Green,
            LsColors_Color::Yellow => Color::Yellow,
            LsColors_Color::Blue => Color::Blue,
            LsColors_Color::Magenta => Color::Magenta,
            LsColors_Color::Cyan => Color::Cyan,
            LsColors_Color::White => Color::White,

            // Below items are a rough translations to 256 colors as
            // nu-ansi-term do not have bright varients
            LsColors_Color::BrightBlack => Color::Fixed(8),
            LsColors_Color::BrightRed => Color::Fixed(9),
            LsColors_Color::BrightGreen => Color::Fixed(10),
            LsColors_Color::BrightYellow => Color::Fixed(11),
            LsColors_Color::BrightBlue => Color::Fixed(12),
            LsColors_Color::BrightMagenta => Color::Fixed(13),
            LsColors_Color::BrightCyan => Color::Fixed(14),
            LsColors_Color::BrightWhite => Color::Fixed(15),
        }
    }

    Style {
        foreground: style.foreground.as_ref().map(to_nu_ansi_term_color),
        background: style.background.as_ref().map(to_nu_ansi_term_color),
        is_bold: style.font_style.bold,
        is_dimmed: style.font_style.dimmed,
        is_italic: style.font_style.italic,
        is_underline: style.font_style.underline,
        is_blink: style.font_style.slow_blink || style.font_style.rapid_blink,
        is_reverse: style.font_style.reverse,
        is_hidden: style.font_style.hidden,
        is_strikethrough: style.font_style.strikethrough,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Find)
    }
}
