use super::color_config::style_primitive;
use crate::viewers::color_config::get_color_config;
use lscolors::{LsColors, Style};
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, DataSource, IntoPipelineData, PipelineData, PipelineMetadata, ShellError,
    Signature, Span, Value, ValueStream,
};
use nu_table::{StyledString, TextStyle, Theme};
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
        Signature::build("table").category(Category::Viewers)
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
        let color_hm = get_color_config(&config);

        let term_width = if let Some((Width(w), Height(_h))) = terminal_size::terminal_size() {
            (w - 1) as usize
        } else {
            80usize
        };

        match input {
            PipelineData::Value(Value::List { vals, .. }, ..) => {
                let table = convert_to_table(vals, ctrlc, &config)?;

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &color_hm, &config);

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::Stream(stream, metadata) => {
                let stream = match metadata {
                    Some(PipelineMetadata {
                        data_source: DataSource::Ls,
                    }) => {
                        let config = config.clone();
                        let ctrlc = ctrlc.clone();

                        let ls_colors = match stack.get_env_var("LS_COLORS") {
                            Some(s) => LsColors::from_string(&s),
                            None => LsColors::default(),
                        };

                        ValueStream::from_stream(
                            stream.map(move |mut x| match &mut x {
                                Value::Record { cols, vals, .. } => {
                                    let mut idx = 0;

                                    while idx < cols.len() {
                                        if cols[idx] == "name" {
                                            if let Some(Value::String { val: path, span }) =
                                                vals.get(idx)
                                            {
                                                match std::fs::symlink_metadata(&path) {
                                                    Ok(metadata) => {
                                                        let style = ls_colors
                                                            .style_for_path_with_metadata(
                                                                path.clone(),
                                                                Some(&metadata),
                                                            );
                                                        let ansi_style = style
                                                            .map(Style::to_crossterm_style)
                                                            .unwrap_or_default();
                                                        let use_ls_colors = config.use_ls_colors;

                                                        if use_ls_colors {
                                                            vals[idx] = Value::String {
                                                                val: ansi_style
                                                                    .apply(path)
                                                                    .to_string(),
                                                                span: *span,
                                                            };
                                                        }
                                                    }
                                                    Err(_) => {
                                                        let style =
                                                            ls_colors.style_for_path(path.clone());
                                                        let ansi_style = style
                                                            .map(Style::to_crossterm_style)
                                                            .unwrap_or_default();
                                                        let use_ls_colors = config.use_ls_colors;

                                                        if use_ls_colors {
                                                            vals[idx] = Value::String {
                                                                val: ansi_style
                                                                    .apply(path)
                                                                    .to_string(),
                                                                span: *span,
                                                            };
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        idx += 1;
                                    }

                                    x
                                }
                                _ => x,
                            }),
                            ctrlc,
                        )
                    }
                    _ => stream,
                };

                let table = convert_to_table(stream, ctrlc, &config)?;

                if let Some(table) = table {
                    let result = nu_table::draw_table(&table, term_width, &color_hm, &config);

                    Ok(Value::String {
                        val: result,
                        span: call.head,
                    }
                    .into_pipeline_data())
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            PipelineData::Value(Value::Record { cols, vals, .. }, ..) => {
                let mut output = vec![];

                for (c, v) in cols.into_iter().zip(vals.into_iter()) {
                    output.push(vec![
                        StyledString {
                            contents: c,
                            style: TextStyle::default_field(),
                        },
                        StyledString {
                            contents: v.into_abbreviated_string(&config),
                            style: TextStyle::default(),
                        },
                    ])
                }

                let table = nu_table::Table {
                    headers: vec![],
                    data: output,
                    theme: load_theme_from_config(&config),
                };

                let result = nu_table::draw_table(&table, term_width, &color_hm, &config);

                Ok(Value::String {
                    val: result,
                    span: call.head,
                }
                .into_pipeline_data())
            }
            PipelineData::Value(Value::Error { error }, ..) => Err(error),
            PipelineData::Value(Value::CustomValue { val, span }, ..) => {
                let base_pipeline = val.to_base_value(span)?.into_pipeline_data();
                self.run(engine_state, stack, call, base_pipeline)
            }
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

        // Vec of Vec of String1, String2 where String1 is datatype and String2 is value
        let mut data: Vec<Vec<(String, String)>> = Vec::new();

        for (row_num, item) in iter.enumerate() {
            if let Some(ctrlc) = &ctrlc {
                if ctrlc.load(Ordering::SeqCst) {
                    return Ok(None);
                }
            }
            if let Value::Error { error } = item {
                return Err(error);
            }
            // String1 = datatype, String2 = value as string
            let mut row: Vec<(String, String)> = vec![("string".to_string(), row_num.to_string())];

            if headers.is_empty() {
                // if header row is empty, this is probably a list so format it that way
                row.push(("list".to_string(), item.into_abbreviated_string(config)))
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
                        Ok(value) => row.push((
                            (&value.get_type()).to_string(),
                            value.into_abbreviated_string(config),
                        )),
                        Err(_) => row.push(("empty".to_string(), String::new())),
                    }
                }
            }

            data.push(row);
        }

        let color_hm = get_color_config(config);
        Ok(Some(nu_table::Table {
            headers: headers
                .into_iter()
                .map(|x| StyledString {
                    contents: x,
                    style: TextStyle {
                        alignment: nu_table::Alignment::Center,
                        color_style: Some(color_hm["header"]),
                    },
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
                                    contents: y.1,
                                    style: TextStyle {
                                        alignment: nu_table::Alignment::Right,
                                        color_style: Some(color_hm["row_index"]),
                                    },
                                }
                            } else {
                                StyledString {
                                    contents: y.1,
                                    style: style_primitive(&y.0, &color_hm),
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
