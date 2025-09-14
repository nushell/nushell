use indexmap::IndexMap;
use nu_cmd_base::formats::to::delimited::merge_descriptors;
use nu_engine::command_prelude::*;
use nu_protocol::{Config, ast::PathMember};
use std::collections::HashSet;

#[derive(Clone)]
pub struct ToMd;

struct ToMdOptions {
    pretty: bool,
    per_element: bool,
    center: Option<Vec<CellPath>>,
    escape_md: bool,
    escape_html: bool,
}

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
                "Treat each row as markdown syntax element",
                Some('e'),
            )
            .named(
                "center",
                SyntaxShape::List(Box::new(SyntaxShape::CellPath)),
                "Formats the Markdown table to center given columns",
                Some('c'),
            )
            .switch(
                "escape-md",
                "Escapes Markdown special characters",
                Some('m'),
            )
            .switch("escape-html", "Escapes HTML special characters", Some('t'))
            .switch(
                "escape-all",
                "Escapes both Markdown and HTML special characters",
                Some('a'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Convert table into simple Markdown."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Outputs an MD string representing the contents of this table",
                example: "[[foo bar]; [1 2]] | to md",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1 | 2 |",
                )),
            },
            Example {
                description: "Optionally, output a formatted markdown string",
                example: "[[foo bar]; [1 2]] | to md --pretty",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1   | 2   |",
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
            Example {
                description: "Separate list into markdown tables",
                example: "[ {foo: 1, bar: 2} {foo: 3, bar: 4} {foo: 5}] | to md --per-element",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| 1 | 2 |\n| 3 | 4 |\n\n| foo |\n| --- |\n| 5 |",
                )),
            },
            Example {
                description: "Center a column of a markdown table",
                example: "[ {foo: 1, bar: 2} {foo: 3, bar: 4}] | to md --pretty --center [bar]",
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- |:---:|\n| 1   |  2  |\n| 3   |  4  |",
                )),
            },
            Example {
                description: "Escape markdown special characters",
                example: r#"[ {foo: "_1_", bar: "\# 2"} {foo: "[3]", bar: "4|5"}] | to md --escape-md"#,
                result: Some(Value::test_string(
                    "| foo | bar |\n| --- | --- |\n| \\_1\\_ | \\# 2 |\n| \\[3\\] | 4\\|5 |",
                )),
            },
            Example {
                description: "Escape html special characters",
                example: r#"[ {a: p, b: "<p>Welcome to nushell</p>"}] | to md --escape-html"#,
                result: Some(Value::test_string(
                    "| a | b |\n| --- | --- |\n| p | &lt;p&gt;Welcome to nushell&lt;&#x2f;p&gt; |",
                )),
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
        let escape_md = call.has_flag(engine_state, stack, "escape-md")?;
        let escape_html = call.has_flag(engine_state, stack, "escape-html")?;
        let escape_both = call.has_flag(engine_state, stack, "escape-all")?;
        let center: Option<Vec<CellPath>> = call.get_flag(engine_state, stack, "center")?;

        let config = stack.get_config(engine_state);

        to_md(
            input,
            ToMdOptions {
                pretty,
                per_element,
                center,
                escape_md: escape_md || escape_both,
                escape_html: escape_html || escape_both,
            },
            &config,
            head,
        )
    }
}

fn to_md(
    input: PipelineData,
    options: ToMdOptions,
    config: &Config,
    head: Span,
) -> Result<PipelineData, ShellError> {
    // text/markdown became a valid mimetype with rfc7763
    let metadata = input
        .metadata()
        .unwrap_or_default()
        .with_content_type(Some("text/markdown".into()));

    let (grouped_input, single_list) = group_by(input, head, config);
    if options.per_element || single_list {
        return Ok(Value::string(
            grouped_input
                .into_iter()
                .map(move |val| match val {
                    Value::List { .. } => {
                        format!(
                            "{}\n\n",
                            table(
                                val.into_pipeline_data(),
                                options.pretty,
                                &options.center,
                                options.escape_md,
                                options.escape_html,
                                config
                            )
                        )
                    }
                    other => fragment(
                        other,
                        options.pretty,
                        &options.center,
                        options.escape_md,
                        options.escape_html,
                        config,
                    ),
                })
                .collect::<Vec<String>>()
                .join("")
                .trim(),
            head,
        )
        .into_pipeline_data_with_metadata(Some(metadata)));
    }
    Ok(Value::string(
        table(
            grouped_input,
            options.pretty,
            &options.center,
            options.escape_md,
            options.escape_html,
            config,
        ),
        head,
    )
    .into_pipeline_data_with_metadata(Some(metadata)))
}

fn escape_markdown_characters(input: String, escape_md: bool, for_table: bool) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        let must_escape = match ch {
            '\\' => true,
            '|' if for_table => true,
            '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '<' | '>' | '#' | '+' | '-'
            | '.' | '!'
                if escape_md =>
            {
                true
            }
            _ => false,
        };

        if must_escape {
            output.push('\\');
        }
        output.push(ch);
    }
    output
}

fn fragment(
    input: Value,
    pretty: bool,
    center: &Option<Vec<CellPath>>,
    escape_md: bool,
    escape_html: bool,
    config: &Config,
) -> String {
    let mut out = String::new();

    if let Value::Record { val, .. } = &input {
        match val.get_index(0) {
            Some((header, data)) if val.len() == 1 => {
                let markup = match header.to_ascii_lowercase().as_ref() {
                    "h1" => "# ".to_string(),
                    "h2" => "## ".to_string(),
                    "h3" => "### ".to_string(),
                    "blockquote" => "> ".to_string(),
                    _ => {
                        return table(
                            input.into_pipeline_data(),
                            pretty,
                            center,
                            escape_md,
                            escape_html,
                            config,
                        );
                    }
                };

                let value_string = data.to_expanded_string("|", config);
                out.push_str(&markup);
                out.push_str(&escape_markdown_characters(
                    if escape_html {
                        v_htmlescape::escape(&value_string).to_string()
                    } else {
                        value_string
                    },
                    escape_md,
                    false,
                ));
            }
            _ => {
                out = table(
                    input.into_pipeline_data(),
                    pretty,
                    center,
                    escape_md,
                    escape_html,
                    config,
                )
            }
        }
    } else {
        let value_string = input.to_expanded_string("|", config);
        out = escape_markdown_characters(
            if escape_html {
                v_htmlescape::escape(&value_string).to_string()
            } else {
                value_string
            },
            escape_md,
            false,
        );
    }

    out.push('\n');
    out
}

fn collect_headers(headers: &[String], escape_md: bool) -> (Vec<String>, Vec<usize>) {
    let mut escaped_headers: Vec<String> = Vec::new();
    let mut column_widths: Vec<usize> = Vec::new();

    if !headers.is_empty() && (headers.len() > 1 || !headers[0].is_empty()) {
        for header in headers {
            let escaped_header_string = escape_markdown_characters(
                v_htmlescape::escape(header).to_string(),
                escape_md,
                true,
            );
            column_widths.push(escaped_header_string.len());
            escaped_headers.push(escaped_header_string);
        }
    } else {
        column_widths = vec![0; headers.len()];
    }

    (escaped_headers, column_widths)
}

fn table(
    input: PipelineData,
    pretty: bool,
    center: &Option<Vec<CellPath>>,
    escape_md: bool,
    escape_html: bool,
    config: &Config,
) -> String {
    let vec_of_values = input
        .into_iter()
        .flat_map(|val| match val {
            Value::List { vals, .. } => vals,
            other => vec![other],
        })
        .collect::<Vec<Value>>();
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

    let (escaped_headers, mut column_widths) = collect_headers(&headers, escape_md);

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
                    let escaped_string = escape_markdown_characters(
                        if escape_html {
                            v_htmlescape::escape(&value_string).to_string()
                        } else {
                            value_string
                        },
                        escape_md,
                        true,
                    );

                    let new_column_width = escaped_string.len();
                    escaped_row.push(escaped_string);

                    if column_widths[i] < new_column_width {
                        column_widths[i] = new_column_width;
                    }
                    if column_widths[i] < 3 {
                        column_widths[i] = 3;
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

    if (column_widths.is_empty() || column_widths.iter().all(|x| *x == 0))
        && escaped_rows.is_empty()
    {
        String::from("")
    } else {
        get_output_string(
            &escaped_headers,
            &escaped_rows,
            &column_widths,
            pretty,
            center,
        )
        .trim()
        .to_string()
    }
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
    center: &Option<Vec<CellPath>>,
) -> String {
    let mut output_string = String::new();

    let mut to_center: HashSet<String> = HashSet::new();
    if let Some(center_vec) = center.as_ref() {
        for cell_path in center_vec {
            if let Some(PathMember::String { val, .. }) = cell_path
                .members
                .iter()
                .find(|member| matches!(member, PathMember::String { .. }))
            {
                to_center.insert(val.clone());
            }
        }
    }

    if !headers.is_empty() {
        output_string.push('|');

        for i in 0..headers.len() {
            output_string.push(' ');
            if pretty {
                if center.is_some() && to_center.contains(&headers[i]) {
                    output_string.push_str(&get_centered_string(
                        headers[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                } else {
                    output_string.push_str(&get_padded_string(
                        headers[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                }
            } else {
                output_string.push_str(&headers[i]);
            }

            output_string.push_str(" |");
        }

        output_string.push_str("\n|");

        for i in 0..headers.len() {
            let centered_column = center.is_some() && to_center.contains(&headers[i]);
            let border_char = if centered_column { ':' } else { ' ' };
            if pretty {
                output_string.push(border_char);
                output_string.push_str(&get_padded_string(
                    String::from("-"),
                    column_widths[i],
                    '-',
                ));
                output_string.push(border_char);
            } else if centered_column {
                output_string.push_str(":---:");
            } else {
                output_string.push_str(" --- ");
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
            if !headers.is_empty() {
                output_string.push(' ');
            }

            if pretty && column_widths.get(i).is_some() {
                if center.is_some() && to_center.contains(&headers[i]) {
                    output_string.push_str(&get_centered_string(
                        row[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                } else {
                    output_string.push_str(&get_padded_string(
                        row[i].clone(),
                        column_widths[i],
                        ' ',
                    ));
                }
            } else {
                output_string.push_str(&row[i]);
            }

            if !headers.is_empty() {
                output_string.push_str(" |");
            }
        }

        output_string.push('\n');
    }

    output_string
}

fn get_centered_string(text: String, desired_length: usize, padding_character: char) -> String {
    let total_padding = if text.len() > desired_length {
        0
    } else {
        desired_length - text.len()
    };

    let repeat_left = total_padding / 2;
    let repeat_right = total_padding - repeat_left;

    format!(
        "{}{}{}",
        padding_character.to_string().repeat(repeat_left),
        text,
        padding_character.to_string().repeat(repeat_right)
    )
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
    use nu_protocol::{Config, IntoPipelineData, Value, casing::Casing, record};

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

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "# Ecuador\n"
        );
    }

    #[test]
    fn render_h2() {
        let value = Value::test_record(record! {
            "H2" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "## Ecuador\n"
        );
    }

    #[test]
    fn render_h3() {
        let value = Value::test_record(record! {
            "H3" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "### Ecuador\n"
        );
    }

    #[test]
    fn render_blockquote() {
        let value = Value::test_record(record! {
            "BLOCKQUOTE" => Value::test_string("Ecuador"),
        });

        assert_eq!(
            fragment(value, false, &None, false, false, &Config::default()),
            "> Ecuador\n"
        );
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
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | country |
            | --- |
            | Ecuador |
            | New Zealand |
            | USA |
            "#)
        );

        assert_eq!(
            table(
                value.into_pipeline_data(),
                true,
                &None,
                false,
                false,
                &Config::default()
            ),
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
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            |  | foo |
            | --- | --- |
            | 1 | 2 |
            | 3 | 4 |
            "#)
        );
    }

    #[test]
    fn test_empty_row_value() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "foo" => Value::test_string("1"),
                "bar" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("3"),
                "bar" => Value::test_string("4"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("5"),
                "bar" => Value::test_string(""),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- | --- |
            | 1 | 2 |
            | 3 | 4 |
            | 5 |  |
            "#)
        );
    }

    #[test]
    fn test_center_column() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "foo" => Value::test_string("1"),
                "bar" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("3"),
                "bar" => Value::test_string("4"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("5"),
                "bar" => Value::test_string("6"),
            }),
        ]);

        let center_columns = Value::test_list(vec![Value::test_cell_path(CellPath {
            members: vec![PathMember::test_string(
                "bar".into(),
                false,
                Casing::Sensitive,
            )],
        })]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        // With pretty
        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- |:---:|
            | 1   |  2  |
            | 3   |  4  |
            | 5   |  6  |
            "#)
        );

        // Without pretty
        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- |:---:|
            | 1 | 2 |
            | 3 | 4 |
            | 5 | 6 |
            "#)
        );
    }

    #[test]
    fn test_empty_center_column() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "foo" => Value::test_string("1"),
                "bar" => Value::test_string("2"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("3"),
                "bar" => Value::test_string("4"),
            }),
            Value::test_record(record! {
                "foo" => Value::test_string("5"),
                "bar" => Value::test_string("6"),
            }),
        ]);

        let center: Option<Vec<CellPath>> = Some(vec![]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | foo | bar |
            | --- | --- |
            | 1   | 2   |
            | 3   | 4   |
            | 5   | 6   |
            "#)
        );
    }

    #[test]
    fn test_center_multiple_columns() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "command" => Value::test_string("ls"),
                "input" => Value::test_string("."),
                "output" => Value::test_string("file.txt"),
            }),
            Value::test_record(record! {
                "command" => Value::test_string("echo"),
                "input" => Value::test_string("'hi'"),
                "output" => Value::test_string("hi"),
            }),
            Value::test_record(record! {
                "command" => Value::test_string("cp"),
                "input" => Value::test_string("a.txt"),
                "output" => Value::test_string("b.txt"),
            }),
        ]);

        let center_columns = Value::test_list(vec![
            Value::test_cell_path(CellPath {
                members: vec![PathMember::test_string(
                    "command".into(),
                    false,
                    Casing::Sensitive,
                )],
            }),
            Value::test_cell_path(CellPath {
                members: vec![PathMember::test_string(
                    "output".into(),
                    false,
                    Casing::Sensitive,
                )],
            }),
        ]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | command | input |  output  |
            |:-------:| ----- |:--------:|
            |   ls    | .     | file.txt |
            |  echo   | 'hi'  |    hi    |
            |   cp    | a.txt |  b.txt   |
            "#)
        );
    }

    #[test]
    fn test_center_non_existing_column() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "name" => Value::test_string("Alice"),
                "age" => Value::test_string("30"),
            }),
            Value::test_record(record! {
                "name" => Value::test_string("Bob"),
                "age" => Value::test_string("5"),
            }),
            Value::test_record(record! {
                "name" => Value::test_string("Charlie"),
                "age" => Value::test_string("20"),
            }),
        ]);

        let center_columns = Value::test_list(vec![Value::test_cell_path(CellPath {
            members: vec![PathMember::test_string(
                "none".into(),
                false,
                Casing::Sensitive,
            )],
        })]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | name    | age |
            | ------- | --- |
            | Alice   | 30  |
            | Bob     | 5   |
            | Charlie | 20  |
            "#)
        );
    }

    #[test]
    fn test_center_complex_cell_path() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "k" => Value::test_string("version"),
                "v" => Value::test_string("0.104.1"),
            }),
            Value::test_record(record! {
                "k" => Value::test_string("build_time"),
                "v" => Value::test_string("2025-05-28 11:00:45 +01:00"),
            }),
        ]);

        let center_columns = Value::test_list(vec![Value::test_cell_path(CellPath {
            members: vec![
                PathMember::test_int(1, false),
                PathMember::test_string("v".into(), false, Casing::Sensitive),
            ],
        })]);

        let cell_path: Vec<CellPath> = center_columns
            .into_list()
            .unwrap()
            .into_iter()
            .map(|v| v.into_cell_path().unwrap())
            .collect();

        let center: Option<Vec<CellPath>> = Some(cell_path);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &center,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | k          |             v              |
            | ---------- |:--------------------------:|
            | version    |          0.104.1           |
            | build_time | 2025-05-28 11:00:45 +01:00 |
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

        let cmd = "{a: 1 b: 2} | to md  | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        );
        assert_eq!(
            Value::test_string("text/markdown"),
            result.expect("There should be a result")
        );
    }

    #[test]
    fn test_escape_md_characters() {
        let value = Value::test_list(vec![
            Value::test_record(record! {
                "name|label" => Value::test_string("orderColumns"),
                "type*" => Value::test_string("'asc' | 'desc' | 'none'"),
            }),
            Value::test_record(record! {
                "name|label" => Value::test_string("_ref_value"),
                "type*" => Value::test_string("RefObject<SampleTableRef | null>"),
            }),
            Value::test_record(record! {
                "name|label" => Value::test_string("onChange"),
                "type*" => Value::test_string("(val: string) => void\\"),
            }),
        ]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | name\|label | type* |
            | --- | --- |
            | orderColumns | 'asc' \| 'desc' \| 'none' |
            | _ref_value | RefObject<SampleTableRef \| null> |
            | onChange | (val: string) => void\\ |
            "#)
        );

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                true,
                false,
                &Config::default()
            ),
            one(r#"
            | name\|label | type\* |
            | --- | --- |
            | orderColumns | 'asc' \| 'desc' \| 'none' |
            | \_ref\_value | RefObject\<SampleTableRef \| null\> |
            | onChange | \(val: string\) =\> void\\ |
            "#)
        );

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                true,
                &None,
                false,
                false,
                &Config::default()
            ),
            one(r#"
            | name\|label  | type*                             |
            | ------------ | --------------------------------- |
            | orderColumns | 'asc' \| 'desc' \| 'none'         |
            | _ref_value   | RefObject<SampleTableRef \| null> |
            | onChange     | (val: string) => void\\           |
            "#)
        );

        assert_eq!(
            table(
                value.into_pipeline_data(),
                true,
                &None,
                true,
                false,
                &Config::default()
            ),
            one(r#"
            | name\|label  | type\*                              |
            | ------------ | ----------------------------------- |
            | orderColumns | 'asc' \| 'desc' \| 'none'           |
            | \_ref\_value | RefObject\<SampleTableRef \| null\> |
            | onChange     | \(val: string\) =\> void\\          |
            "#)
        );
    }

    #[test]
    fn test_escape_html_characters() {
        let value = Value::test_list(vec![Value::test_record(record! {
            "tag" => Value::test_string("table"),
            "code" => Value::test_string(r#"<table><tr><td scope="row">Chris</td><td>HTML tables</td><td>22</td></tr><tr><td scope="row">Dennis</td><td>Web accessibility</td><td>45</td></tr></table>"#),
        })]);

        assert_eq!(
            table(
                value.clone().into_pipeline_data(),
                false,
                &None,
                false,
                true,
                &Config::default()
            ),
            one(r#"
            | tag | code |
            | --- | --- |
            | table | &lt;table&gt;&lt;tr&gt;&lt;td scope=&quot;row&quot;&gt;Chris&lt;&#x2f;td&gt;&lt;td&gt;HTML tables&lt;&#x2f;td&gt;&lt;td&gt;22&lt;&#x2f;td&gt;&lt;&#x2f;tr&gt;&lt;tr&gt;&lt;td scope=&quot;row&quot;&gt;Dennis&lt;&#x2f;td&gt;&lt;td&gt;Web accessibility&lt;&#x2f;td&gt;&lt;td&gt;45&lt;&#x2f;td&gt;&lt;&#x2f;tr&gt;&lt;&#x2f;table&gt; |
            "#)
        );

        assert_eq!(
            table(
                value.into_pipeline_data(),
                true,
                &None,
                false,
                true,
                &Config::default()
            ),
            one(r#"
            | tag   | code                                                                                                                                                                                                                                                                                                                                    |
            | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
            | table | &lt;table&gt;&lt;tr&gt;&lt;td scope=&quot;row&quot;&gt;Chris&lt;&#x2f;td&gt;&lt;td&gt;HTML tables&lt;&#x2f;td&gt;&lt;td&gt;22&lt;&#x2f;td&gt;&lt;&#x2f;tr&gt;&lt;tr&gt;&lt;td scope=&quot;row&quot;&gt;Dennis&lt;&#x2f;td&gt;&lt;td&gt;Web accessibility&lt;&#x2f;td&gt;&lt;td&gt;45&lt;&#x2f;td&gt;&lt;&#x2f;tr&gt;&lt;&#x2f;table&gt; |
            "#)
        );
    }
}
