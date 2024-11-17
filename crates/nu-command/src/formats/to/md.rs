use indexmap::IndexMap;
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_engine::command_prelude::*;
use nu_protocol::Config;

#[derive(Clone)]
pub struct ToMd;

impl Command for ToMd {
    fn name(&self) -> &str {
        "to md"
    }

    fn signature(&self) -> Signature {
        Signature::build("to md")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "pretty",
                "Formats the Markdown table to vertically align items",
                Some('p'),
            )
            .switch(
                "per-element",
                "treat each row as markdown syntax element",
                Some('e'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into simple Markdown."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Outputs an MD string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to md",
                result: Some(Value::test_string("|foo|bar|\n|-|-|\n|1|2|\n")),
            },
            Example {
                description: "Optionally, output a formatted markdown string",
                example: "[[foo bar]; [1 2]] | to md --pretty",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1   | 2   |\n",
                )),
            },
            Example {
                description: "Treat each row as a markdown element",
                example: r#"[{"H1": "Welcome to Nushell" } [[foo bar]; [1 2]]] | to md --per-element --pretty"#,
                result: Some(Value::test_string(
                    "# Welcome to Nushell\n| foo | bar |\n| --- | --- |\n| 1   | 2   |",
                )),
            },
            Example {
                description: "Render a list",
                example: "[0 1 2] | to md --pretty",
                result: Some(Value::test_string("0\n1\n2")),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let head = call.head;
        let pretty = call.has_flag(engine_state, stack, "pretty")?;
        let per_element = call.has_flag(engine_state, stack, "per-element")?;
        let config = stack.get_config(engine_state);
        to_md(input, pretty, per_element, &config, head)
    }
}

fn to_md(
    input: PipelineData,
    pretty: bool,
    per_element: bool,
    config: &Config,
    head: Span,
) -> Result<PipelineData, ShellError> {
    // text/markdown became a valid mimetype with rfc7763
    let metadata = input
        .metadata()
        .unwrap_or_default()
        .with_content_type(Some("text/markdown".into()));

    let (grouped_input, single_list) = group_by(input, head, config);
    if per_element || single_list {
        return Ok(Value::string(
            grouped_input
                .into_iter()
                .map(move |val| match val {
                    Value::List { .. } => table(val.into_pipeline_data(), pretty, config),
                    other => fragment(other, pretty, config),
                })
                .collect::<Vec<String>>()
                .join(""),
            head,
        )
        .into_pipeline_data_with_metadata(Some(metadata)));
    }
    Ok(Value::string(table(grouped_input, pretty, config), head)
        .into_pipeline_data_with_metadata(Some(metadata)))
}

fn fragment(input: Value, pretty: bool, config: &Config) -> String {
    let mut out = String::new();

    if let Value::Record { val, .. } = &input {
        match val.get_index(0) {
            Some((header, data)) if val.len() == 1 => {
                let markup = match header.to_ascii_lowercase().as_ref() {
                    "h1" => "# ".to_string(),
                    "h2" => "## ".to_string(),
                    "h3" => "### ".to_string(),
                    "blockquote" => "> ".to_string(),
                    _ => return table(input.into_pipeline_data(), pretty, config),
                };

                out.push_str(&markup);
                out.push_str(&data.to_expanded_string("|", config));
            }
            _ => out = table(input.into_pipeline_data(), pretty, config),
        }
    } else {
        out = input.to_expanded_string("|", config)
    }

    out.push('\n');
    out
}

fn collect_headers(headers: &[String]) -> (Vec<String>, Vec<usize>) {
    let mut escaped_headers: Vec<String> = Vec::new();
    let mut column_widths: Vec<usize> = Vec::new();

    if !headers.is_empty() && (headers.len() > 1 || !headers[0].is_empty()) {
        for header in headers {
            let escaped_header_string = v_htmlescape::escape(header).to_string();
            column_widths.push(escaped_header_string.len());
            escaped_headers.push(escaped_header_string);
        }
    } else {
        column_widths = vec![0; headers.len()]
    }

    (escaped_headers, column_widths)
}

fn table(input: PipelineData, pretty: bool, config: &Config) -> String {
    let vec_of_values = input.into_iter().collect::<Vec<Value>>();
    let mut headers = merge_descriptors(&vec_of_values);

    let mut empty_header_index = 0;
    for value in &vec_of_values {
        if let Value::Record { val, .. } = value {
            for column in val.columns() {
                if column.is_empty() && !headers.contains(&String::new()) {
                    headers.insert(empty_header_index, String::new());
                    empty_header_index += 1;
                    break;
                }
                empty_header_index += 1;
            }
        }
    }

    let (escaped_headers, mut column_widths) = collect_headers(&headers);

    let mut escaped_rows: Vec<Vec<String>> = Vec::new();

    for row in vec_of_values {
        let mut escaped_row: Vec<String> = Vec::new();
        let span = row.span();

        match row.to_owned() {
            Value::Record { val: row, .. } => {
                for i in 0..headers.len() {
                    let value_string = row
                        .get(&headers[i])
                        .cloned()
                        .unwrap_or_else(|| Value::nothing(span))
                        .to_expanded_string(", ", config);
                    let new_column_width = value_string.len();

                    escaped_row.push(value_string);

                    if column_widths[i] < new_column_width {
                        column_widths[i] = new_column_width;
                    }
                }
            }
            p => {
                let value_string =
                    v_htmlescape::escape(&p.to_abbreviated_string(config)).to_string();
                escaped_row.push(value_string);
            }
        }

        escaped_rows.push(escaped_row);
    }

    let output_string = if (column_widths.is_empty() || column_widths.iter().all(|x| *x == 0))
        && escaped_rows.is_empty()
    {
        String::from("")
    } else {
        get_output_string(&escaped_headers, &escaped_rows, &column_widths, pretty)
            .trim()
            .to_string()
    };

    output_string
}

pub fn group_by(values: PipelineData, head: Span, config: &Config) -> (PipelineData, bool) {
    let mut lists = IndexMap::new();
    let mut single_list = false;
    for val in values {
        if let Value::Record {
            val: ref record, ..
        } = val
        {
            lists
                .entry(record.columns().map(|c| c.as_str()).collect::<String>())
                .and_modify(|v: &mut Vec<Value>| v.push(val.clone()))
                .or_insert_with(|| vec![val.clone()]);
        } else {
            lists
                .entry(val.to_expanded_string(",", config))
                .and_modify(|v: &mut Vec<Value>| v.push(val.clone()))
                .or_insert_with(|| vec![val.clone()]);
        }
    }
    let mut output = vec![];
    for (_, mut value) in lists {
        if value.len() == 1 {
            output.push(value.pop().unwrap_or_else(|| Value::nothing(head)))
        } else {
            output.push(Value::list(value.to_vec(), head))
        }
    }
    if output.len() == 1 {
        single_list = true;
    }
    (Value::list(output, head).into_pipeline_data(), single_list)
}

fn get_output_string(
    headers: &[String],
    rows: &[Vec<String>],
    column_widths: &[usize],
    pretty: bool,
) -> String {
    let mut output_string = String::new();

    if !headers.is_empty() {
        output_string.push('|');

        for i in 0..headers.len() {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(
                    headers[i].clone(),
                    column_widths[i],
                    ' ',
                ));
                output_string.push(' ');
            } else {
                output_string.push_str(&headers[i]);
            }

            output_string.push('|');
        }

        output_string.push_str("\n|");

        for &col_width in column_widths.iter().take(headers.len()) {
            if pretty {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(String::from("-"), col_width, '-'));
                output_string.push(' ');
            } else {
                output_string.push('-');
            }

            output_string.push('|');
        }

        output_string.push('\n');
    }

    for row in rows {
        if !headers.is_empty() {
            output_string.push('|');
        }

        for i in 0..row.len() {
            if pretty && column_widths.get(i).is_some() {
                output_string.push(' ');
                output_string.push_str(&get_padded_string(row[i].clone(), column_widths[i], ' '));
                output_string.push(' ');
            } else {
                output_string.push_str(&row[i]);
            }

            if !headers.is_empty() {
                output_string.push('|');
            }
        }

        output_string.push('\n');
    }

    output_string
}

fn get_padded_string(text: String, desired_length: usize, padding_character: char) -> String {
    let repeat_length = if text.len() > desired_length {
        0
    } else {
        desired_length - text.len()
    };

    format!(
        "{}{}",
        text,
        padding_character.to_string().repeat(repeat_length)
    )
}

#[cfg(test)]
mod tests {
    use crate::{Get, Metadata};

    use super::*;
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;
    use nu_protocol::{record, Config, IntoPipelineData, Value};

    fn one(string: &str) -> String {
        string
            .lines()
            .skip(1)
            .map(|line| line.trim())
            .collect::<Vec<&str>>()
            .join("\n")
            .trim_end()
            .to_string()
    }

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ToMd {})
    }

    #[test]
    fn render_h1() {
        let value = Value::test_record(record! {
            "H1" => Value::test_string("Ecuador"),
        });

        assert_eq!(fragment(value, false, &Config::default()), "# Ecuador\n");
    }

    #[test]
    fn render_h2() {
        let value = Value::test_record(record! {
            "H2" => Value::test_string("Ecuador"),
        });

        assert_eq!(fragment(value, false, &Config::default()), "## Ecuador\n");
    }

    #[test]
    fn render_h3() {
        let value = Value::test_record(record! {
            "H3" => Value::test_string("Ecuador"),
        });

        assert_eq!(fragment(value, false, &Config::default()), "### Ecuador\n");
    }

    #[test]
    fn render_blockquote() {
        let value = Value::test_record(record! {
            "BLOCKQUOTE" => Value::test_string("Ecuador"),
        });

        assert_eq!(fragment(value, false, &Config::default()), "> Ecuador\n");
    }

    #[test]
    fn render_table() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "country" => Value::test_string("Ecuador"),
            }),
            Value::test_record(record! {
                "country" => Value::test_string("New Zealand"),
            }),
            Value::test_record(record! {
                "country" => Value::test_string("USA"),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &Config::default()
            ),
            one(r#"
            |country|
            |-|
            |Ecuador|
            |New Zealand|
            |USA|
        "#)
        );

        assert_eq!(
            table(value.into_pipeline_data(), true, &Config::default()),
            one(r#"
            | country     |
            | ----------- |
            | Ecuador     |
            | New Zealand |
            | USA         |
        "#)
        );
    }

    #[test]
    fn test_empty_column_header() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "" => Value::test_string("1"),
                "foo" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "" => Value::test_string("3"),
                "foo" => Value::test_string("4"),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &Config::default()
            ),
            one(r#"
            ||foo|
            |-|-|
            |1|2|
            |3|4|
        "#)
        );
    }

    #[test]
    fn test_content_type_metadata() {
        let mut engine_state = Box::new(EngineState::new());
        let state_delta = {
            // Base functions that are needed for testing
            // Try to keep this working set small to keep tests running as fast as possible
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(ToMd {}));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };
        let delta = state_delta;

        engine_state
            .merge_delta(delta)
            .expect("Error merging delta");

        let cmd = "{a: 1 b: 2} | to md  | metadata | get content_type";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_record(record!("content_type" => Value::test_string("text/markdown"))),
            result.expect("There should be a result")
        );
    }
}
