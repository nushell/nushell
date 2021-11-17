use chrono::{DateTime, Utc};
use lscolors::{LsColors, Style};
use nu_engine::eval_expression;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, PipelineData, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Ls;

//NOTE: this is not a real implementation :D. It's just a simple one to test with until we port the real one.
impl Command for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn usage(&self) -> &str {
        "List the files in a directory."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("ls")
            .optional(
                "pattern",
                SyntaxShape::GlobPattern,
                "the glob pattern to use",
            )
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let config = stack.get_config()?;
        let pattern = if let Some(expr) = call.positional.get(0) {
            let result = eval_expression(engine_state, stack, expr)?;
            let mut result = result.as_string()?;

            let path = std::path::Path::new(&result);
            if path.is_dir() {
                if !result.ends_with(std::path::MAIN_SEPARATOR) {
                    result.push(std::path::MAIN_SEPARATOR);
                }
                result.push('*');
            }

            result
        } else {
            "*".into()
        };

        let call_span = call.head;
        let glob = glob::glob(&pattern).unwrap();
        let ls_colors = LsColors::from_env().unwrap_or_default();

        Ok(glob
            .into_iter()
            .map(move |x| match x {
                Ok(path) => match std::fs::symlink_metadata(&path) {
                    Ok(metadata) => {
                        let is_file = metadata.is_file();
                        let is_dir = metadata.is_dir();
                        let filesize = metadata.len();
                        let mut cols = vec!["name".into(), "type".into(), "size".into()];
                        let style = ls_colors.style_for_path(path.clone());
                        let ansi_style = style.map(Style::to_crossterm_style).unwrap_or_default();
                        let use_ls_colors = config.use_ls_colors;

                        let mut vals = vec![
                            if use_ls_colors {
                                Value::String {
                                    val: ansi_style.apply(path.to_string_lossy()).to_string(),
                                    span: call_span,
                                }
                            } else {
                                Value::String {
                                    val: path.to_string_lossy().to_string(),
                                    span: call_span,
                                }
                            },
                            if is_file {
                                Value::string("file", call_span)
                            } else if is_dir {
                                Value::string("dir", call_span)
                            } else {
                                Value::Nothing { span: call_span }
                            },
                            Value::Filesize {
                                val: filesize as i64,
                                span: call_span,
                            },
                        ];

                        if let Ok(date) = metadata.modified() {
                            let utc: DateTime<Utc> = date.into();

                            cols.push("modified".into());
                            vals.push(Value::Date {
                                val: utc.into(),
                                span: call_span,
                            });
                        }

                        Value::Record {
                            cols,
                            vals,
                            span: call_span,
                        }
                    }
                    Err(_) => {
                        let style = ls_colors.style_for_path(path.clone());
                        let ansi_style = style.map(Style::to_crossterm_style).unwrap_or_default();
                        let use_ls_colors = config.use_ls_colors;

                        Value::Record {
                            cols: vec!["name".into(), "type".into(), "size".into()],
                            vals: vec![
                                if use_ls_colors {
                                    Value::String {
                                        val: ansi_style.apply(path.to_string_lossy()).to_string(),
                                        span: call_span,
                                    }
                                } else {
                                    Value::String {
                                        val: path.to_string_lossy().to_string(),
                                        span: call_span,
                                    }
                                },
                                Value::Nothing { span: call_span },
                                Value::Nothing { span: call_span },
                            ],
                            span: call_span,
                        }
                    }
                },
                _ => Value::Nothing { span: call_span },
            })
            .into_pipeline_data(engine_state.ctrlc.clone()))
    }
}
