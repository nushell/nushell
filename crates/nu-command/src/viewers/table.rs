use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Config, IntoPipelineData, PipelineData, ShellError, Signature, Span, Value};
use nu_table::{StyledString, Theme};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use terminal_size::{Height, Width};

#[derive(Clone)]
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let ctrlc = engine_state.ctrlc.clone();
        let config = stack.get_config()?;

        let term_width = if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
            w as usize
        } else {
            80usize
        };

        match input {
            PipelineData::Value(Value::List { vals, .. }) => {
                let table = convert_to_table(vals, ctrlc, &config)?;

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &HashMap::new());

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::Stream(stream) => {
                let table = convert_to_table(stream, ctrlc, &config)?;

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &HashMap::new());

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::Value(Value::Record { cols, vals, .. }) => {
                let mut output = vec![];

                for (c, v) in cols.into_iter().zip(vals.into_iter()) {
                    output.push(vec![
                        StyledString {
                            contents: c,
                            style: nu_table::TextStyle::default_field(),
                        },
                        StyledString {
                            contents: v.into_string(", ", &config),
                            style: nu_table::TextStyle::default(),
                        },
                    ])
                }

                let table = nu_table::Table {
                    headers: vec![],
                    data: output,
                    theme: load_theme_from_config(&config),
                };

                let result = nu_table::draw_table(&table, term_width, &HashMap::new());

                Ok(Value::String {
                    val: result,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(Value::Error { error }) => Err(error),
            x => Ok(x),
        }
    }
}

fn convert_to_table(
    iter: impl IntoIterator<Item = Value>,
    ctrlc: Option<Arc<AtomicBool>>,
    config: &Config,
) -> Result<Option<nu_table::Table>, ShellError> {
    let mut iter = iter.into_iter().peekable();

    if let Some(first) = iter.peek() {
        let mut headers = first.columns();

        if !headers.is_empty() {
            headers.insert(0, "#".into());
        }

        let mut data = vec![];

        for (row_num, item) in iter.enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }
            if let Value::Error { error } = item {
                return Err(error);
            }
            let mut row = vec![row_num.to_string()];

            if headers.is_empty() {
                row.push(item.into_string(", ", config))
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
                        Ok(value) => row.push(value.into_string(", ", config)),
                        Err(_) => row.push(String::new()),
                    }
                }
            }

            data.push(row);
        }

        Ok(Some(nu_table::Table {
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
            theme: load_theme_from_config(config),
        }))
    } else {
        Ok(None)
    }
}

fn load_theme_from_config(config: &Config) -> Theme {
    match config.table_mode.as_str() {
        "basic" => nu_table::Theme::basic(),
        "compact" => nu_table::Theme::compact(),
        "compact_double" => nu_table::Theme::compact_double(),
        "light" => nu_table::Theme::light(),
        "with_love" => nu_table::Theme::with_love(),
        "rounded" => nu_table::Theme::rounded(),
        "reinforced" => nu_table::Theme::reinforced(),
        "heavy" => nu_table::Theme::heavy(),
        "none" => nu_table::Theme::none(),
        _ => nu_table::Theme::rounded(),
    }
}
