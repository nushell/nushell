use crate::commands::table::options::{ConfigExtensions, NuConfig as TableConfiguration};
use crate::prelude::*;
use crate::primitive::get_color_config;
use nu_data::value::{format_leaf, style_leaf};
use nu_engine::WholeStreamCommand;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Signature, SyntaxShape, UntaggedValue, Value};
use nu_table::{draw_table, Alignment, StyledString, TextStyle};
use std::collections::HashMap;
use std::time::Instant;

const STREAM_PAGE_SIZE: usize = 1000;
const STREAM_TIMEOUT_CHECK_INTERVAL: usize = 100;

pub struct Command;

#[async_trait]
impl WholeStreamCommand for Command {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        table(TableConfiguration::new(), (args)).await
    }
}

pub fn from_list(
    values: &[Value],
    configuration: &TableConfiguration,
    starting_idx: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> nu_table::Table {
    let header_style = configuration.header_style();
    let mut headers: Vec<StyledString> = nu_protocol::merge_descriptors(values)
        .into_iter()
        .map(|x| StyledString::new(x, header_style))
        .collect();
    let entries = values_to_entries(values, &mut headers, configuration, starting_idx, &color_hm);
    nu_table::Table {
        headers,
        data: entries,
        theme: configuration.table_mode(),
    }
}

fn values_to_entries(
    values: &[Value],
    headers: &mut Vec<StyledString>,
    configuration: &TableConfiguration,
    starting_idx: usize,
    color_hm: &HashMap<String, nu_ansi_term::Style>,
) -> Vec<Vec<StyledString>> {
    let disable_indexes = configuration.disabled_indexes();
    let mut entries = vec![];

    if headers.is_empty() {
        headers.push(StyledString::new("".to_string(), TextStyle::basic_left()));
    }

    for (idx, value) in values.iter().enumerate() {
        let mut row: Vec<StyledString> = headers
            .iter()
            .map(|d: &StyledString| {
                if d.contents.is_empty() {
                    match value {
                        Value {
                            value: UntaggedValue::Row(..),
                            ..
                        } => StyledString::new(
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing(), &color_hm),
                        ),
                        _ => StyledString::new(
                            format_leaf(value).plain_string(100_000),
                            style_leaf(value, &color_hm),
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
                                style_leaf(data.borrow(), &color_hm),
                            )
                        }
                        _ => StyledString::new(
                            format_leaf(&UntaggedValue::nothing()).plain_string(100_000),
                            style_leaf(&UntaggedValue::nothing(), &color_hm),
                        ),
                    }
                }
            })
            .collect();

        // Indices are green, bold, right-aligned:
        // unless we change them :)
        if !disable_indexes {
            row.insert(
                0,
                StyledString::new(
                    (starting_idx + idx).to_string(),
                    TextStyle::new().alignment(Alignment::Right).style(
                        color_hm
                            .get("index_color")
                            .unwrap_or(
                                &nu_ansi_term::Style::default()
                                    .bold()
                                    .fg(nu_ansi_term::Color::Green),
                            )
                            .to_owned(),
                    ),
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
                TextStyle::new()
                    .alignment(Alignment::Center)
                    .fg(nu_ansi_term::Color::Green)
                    .bold(Some(true)),
            ),
        );
    }

    entries
}

async fn table(
    configuration: TableConfiguration,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let mut args = args.evaluate_once().await?;
    let mut finished = false;
    // Ideally, get_color_config would get all the colors configured in the config.toml
    // and create a style based on those settings. However, there are few places where
    // this just won't work right now, like header styling, because a style needs to know
    // more than just color, it needs fg & bg color, bold, dimmed, italic, underline,
    // blink, reverse, hidden, strikethrough and most of those aren't available in the
    // config.toml.... yet.
    let color_hm = get_color_config();

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
            let t = from_list(&input, &configuration, start_number, &color_hm);

            let output = draw_table(&t, term_width, &color_hm);
            println!("{}", output);
        }

        start_number += input.len();
    }

    Ok(OutputStream::empty())
}

#[cfg(test)]
mod tests {
    use super::Command;
    use super::ShellError;

    #[test]
    fn examples_work_as_expected() -> Result<(), ShellError> {
        use crate::examples::test as test_examples;

        test_examples(Command {})
    }
}
