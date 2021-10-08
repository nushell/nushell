use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, Span, Value};
use nu_table::StyledString;
use std::collections::HashMap;
use terminal_size::{Height, Width};

pub struct Table;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Table {
    fn name(&self) -> &str {
        "table"
    }

    fn usage(&self) -> &str {
        "Render the table."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("table")
    }

    fn run(
        &self,
        _context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let term_width = if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
            w as usize
        } else {
            80usize
        };

        match input {
            Value::List { vals, .. } => {
                let table = convert_to_table(vals);

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &HashMap::new());

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    })
                } else {
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Stream { stream, .. } => {
                let table = convert_to_table(stream);

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &HashMap::new());

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    })
                } else {
                    Ok(Value::Nothing { span: call.head })
                }
            }
            Value::Record { cols, vals, .. } => {
                let mut output = vec![];

                for (c, v) in cols.into_iter().zip(vals.into_iter()) {
                    output.push(vec![
                        StyledString {
                            contents: c,
                            style: nu_table::TextStyle::default_field(),
                        },
                        StyledString {
                            contents: v.into_string(),
                            style: nu_table::TextStyle::default(),
                        },
                    ])
                }

                let table = nu_table::Table {
                    headers: vec![],
                    data: output,
                    theme: nu_table::Theme::rounded(),
                };

                let result = nu_table::draw_table(&table, term_width, &HashMap::new());

                Ok(Value::String {
                    val: result,
                    span: call.head,
                })
            }
            x => Ok(x),
        }
    }
}

fn convert_to_table(iter: impl IntoIterator<Item = Value>) -> Option<nu_table::Table> {
    let mut iter = iter.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let mut headers = first.columns();

        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        let mut data = vec![];

        for (row_num, item) in iter.enumerate() {
            let mut row = vec![row_num.to_string()];

            if headers.is_empty() {
                row.push(item.into_string())
            } else {
                for header in headers.iter().skip(1) {
                    let result = match item {
                        Value::Record { .. } => {
                            item.clone().follow_cell_path(&[PathMember::String {
                                val: header.into(),
                                span: Span::unknown(),
                            }])
                        }
                        _ => Ok(item.clone()),
                    };

                    match result {
                        Ok(value) => row.push(value.into_string()),
                        Err(_) => row.push(String::new()),
                    }
                }
            }

            data.push(row);
        }

        Some(nu_table::Table {
            headers: headers
                .into_iter()
                .map(|x| StyledString {
                    contents: x,
                    style: nu_table::TextStyle::default_header(),
                })
                .collect(),
            data: data
                .into_iter()
                .map(|x| {
                    x.into_iter()
                        .enumerate()
                        .map(|(col, y)| {
                            if col == 0 {
                                StyledString {
                                    contents: y,
                                    style: nu_table::TextStyle::default_header(),
                                }
                            } else {
                                StyledString {
                                    contents: y,
                                    style: nu_table::TextStyle::basic_left(),
                                }
                            }
                        })
                        .collect::<Vec<StyledString>>()
                })
                .collect(),
            theme: nu_table::Theme::rounded(),
        })
    } else {
        None
    }
}
