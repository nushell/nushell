use crate::commands::WholeStreamCommand;
use crate::data::value::{format_leaf, style_leaf};
use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_table::{draw_table, Alignment, StyledString, TextStyle, Theme};
use std::time::Instant;

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;

pub struct Table;

#[async_trait]
impl WholeStreamCommand for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn signature(&self) -> Signature {
        Signature::build("table").named(
            "start_number",
            SyntaxShape::Number,
            "row number to start viewing from",
            Some('n'),
        )
    }

    fn usage(&self) -> &str {
        "View the contents of the pipeline as a table."
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        table(args, registry).await
    }
}

fn str_to_color(s: String) -> Option<ansi_term::Color> {
    match s.as_str() {
        "g" | "green" => Some(ansi_term::Color::Green),
        "r" | "red" => Some(ansi_term::Color::Red),
        "u" | "blue" => Some(ansi_term::Color::Blue),
        "b" | "black" => Some(ansi_term::Color::Black),
        "y" | "yellow" => Some(ansi_term::Color::Yellow),
        "p" | "purple" => Some(ansi_term::Color::Purple),
        "c" | "cyan" => Some(ansi_term::Color::Cyan),
        "w" | "white" => Some(ansi_term::Color::White),
        _ => None,
    }
}

pub fn from_list(values: &[Value], starting_idx: usize) -> nu_table::Table {
    let config = crate::data::config::config(Tag::unknown());

    let header_style = if let Ok(config) = &config {
        let header_align = config.get("header_align").map_or(Alignment::Left, |a| {
            a.as_string()
                .map_or(Alignment::Center, |a| match a.to_lowercase().as_str() {
                    "center" | "c" => Alignment::Center,
                    "right" | "r" => Alignment::Right,
                    _ => Alignment::Center,
                })
        });

        let header_color = match config.get("header_color") {
            Some(c) => match c.as_string() {
                Ok(color) => str_to_color(color.to_lowercase()).unwrap_or(ansi_term::Color::Green),
                _ => ansi_term::Color::Green,
            },
            _ => ansi_term::Color::Green,
        };

        let header_bold = config
            .get("header_bold")
            .map(|x| x.as_bool().unwrap_or(true))
            .unwrap_or(true);

        TextStyle {
            alignment: header_align,
            color: Some(header_color),
            is_bold: header_bold,
        }
    } else {
        TextStyle::default_header()
    };

    let mut headers: Vec<StyledString> = nu_protocol::merge_descriptors(values)
        .into_iter()
        .map(|x| StyledString::new(x, header_style.clone()))
        .collect();
    let entries = values_to_entries(values, &mut headers, starting_idx);

    if let Ok(config) = config {
        if let Some(style) = config.get("table_mode") {
            if let Ok(table_mode) = style.as_string() {
                if table_mode == "light" {
                    return nu_table::Table {
                        headers,
                        data: entries,
                        theme: Theme::light(),
                    };
                }
            }
        }
    }
    nu_table::Table {
        headers,
        data: entries,
        theme: Theme::compact(),
    }
}

fn are_table_indexes_disabled() -> bool {
    let config = crate::data::config::config(Tag::unknown());
    match config {
        Ok(config) => {
            let disable_indexes = config.get("disable_table_indexes");
            disable_indexes.map_or(false, |x| x.as_bool().unwrap_or(false))
        }
        _ => false,
    }
}

fn values_to_entries(
    values: &[Value],
    headers: &mut Vec<StyledString>,
    starting_idx: usize,
) -> Vec<Vec<StyledString>> {
    let disable_indexes = are_table_indexes_disabled();
    let mut entries = vec![];

    if headers.is_empty() {
        headers.push(StyledString::new("".to_string(), TextStyle::basic()));
    }

    for (idx, value) in values.iter().enumerate() {
        let mut row: Vec<StyledString> = headers
            .iter()
            .map(|d: &StyledString| {
                if d.contents == "" {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => StyledString::new(
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing()),
                        ),
                        _ => StyledString::new(
                            format_leaf(value).plain_string(100_000),
                            style_leaf(value),
                        ),
                    }
                } else {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => {
                            let data = value.get_data(&d.contents);

                            StyledString::new(
                                format_leaf(data.borrow()).plain_string(100_000),
                                style_leaf(data.borrow()),
                            )
                        }
                        _ => StyledString::new(
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing()),
                        ),
                    }
                }
            })
            .collect();

        // Indices are green, bold, right-aligned:
        if !disable_indexes {
            row.insert(
                0,
                StyledString::new(
                    (starting_idx + idx).to_string(),
                    TextStyle {
                        alignment: Alignment::Right,
                        color: Some(ansi_term::Color::Green),
                        is_bold: true,
                    },
                ),
            );
        }

        entries.push(row);
    }

    if !disable_indexes {
        headers.insert(
            0,
            StyledString::new(
                "#".to_owned(),
                TextStyle {
                    alignment: Alignment::Center,
                    color: Some(ansi_term::Color::Green),
                    is_bold: true,
                },
            ),
        );
    }

    entries
}

async fn table(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let mut args = args.evaluate_once(&registry).await?;
    let mut finished = false;

    // let host = args.host.clone();
    let mut start_number = match args.get("start_number") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::Int(i)),
            ..
        }) => {
            if let Some(num) = i.to_usize() {
                num
            } else {
                return Err(ShellError::labeled_error(
                    "Expected a row number",
                    "expected a row number",
                    &args.args.call_info.name_tag,
                ));
            }
        }
        _ => 0,
    };

    let mut delay_slot = None;

    let term_width = args.host.lock().width();

    while !finished {
        let mut new_input: VecDeque<Value> = VecDeque::new();

        let start_time = Instant::now();
        for idx in 0..STREAM_PAGE_SIZE {
            if let Some(val) = delay_slot {
                new_input.push_back(val);
                delay_slot = None;
            } else {
                match args.input.next().await {
                    Some(a) => {
                        if !new_input.is_empty() {
                            if let Some(descs) = new_input.get(0) {
                                let descs = descs.data_descriptors();
                                let compare = a.data_descriptors();
                                if descs != compare {
                                    delay_slot = Some(a);
                                    break;
                                } else {
                                    new_input.push_back(a);
                                }
                            } else {
                                new_input.push_back(a);
                            }
                        } else {
                            new_input.push_back(a);
                        }
                    }
                    _ => {
                        finished = true;
                        break;
                    }
                }

                // Check if we've gone over our buffering threshold
                if (idx + 1) % STREAM_TIMEOUT_CHECK_INTERVAL == 0 {
                    let end_time = Instant::now();

                    // If we've been buffering over a second, go ahead and send out what we have so far
                    if (end_time - start_time).as_secs() >= 1 {
                        break;
                    }
                }
            }
        }

        let input: Vec<Value> = new_input.into();

        if !input.is_empty() {
            let t = from_list(&input, start_number);

            draw_table(&t, term_width);
        }

        start_number += input.len();
    }

    Ok(OutputStream::empty())
}
