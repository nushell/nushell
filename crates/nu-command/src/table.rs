use std::collections::HashMap;

use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, Span, Value};
use nu_table::StyledString;

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
        match input {
            Value::List { vals, .. } => {
                let table = convert_to_table(vals);

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, 80, &HashMap::new());

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
                    let result = nu_table::draw_table(&table, 80, &HashMap::new());

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    })
                } else {
                    Ok(Value::Nothing { span: call.head })
                }
            }
            x => Ok(x),
        }
    }
}

fn convert_to_table(iter: impl IntoIterator<Item = Value>) -> Option<nu_table::Table> {
    let mut iter = iter.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let mut headers = first.columns();
        headers.insert(0, "#".into());

        let mut data = vec![];

        for (row_num, item) in iter.enumerate() {
            let mut row = vec![row_num.to_string()];

            for header in headers.iter().skip(1) {
                let result = if header == "<value>" {
                    Ok(item.clone())
                } else {
                    item.clone().follow_cell_path(&[PathMember::String {
                        val: header.into(),
                        span: Span::unknown(),
                    }])
                };

                match result {
                    Ok(value) => row.push(value.into_string()),
                    Err(_) => row.push(String::new()),
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
