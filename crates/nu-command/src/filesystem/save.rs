use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Value,
};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Clone)]
pub struct Save;

impl Command for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn usage(&self) -> &str {
        "Save a file."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec![
            "write",
            "write_file",
            "append",
            "redirection",
            "file",
            "io",
            ">",
            ">>",
        ]
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("save")
            .required("filename", SyntaxShape::Filepath, "the filename to use")
            .switch("raw", "save file as raw binary", Some('r'))
            .switch("append", "append input to the end of the file", None)
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let raw = call.has_flag("raw");
        let append = call.has_flag("append");

        let span = call.head;

        let path = call.req::<Spanned<String>>(engine_state, stack, 0)?;
        let arg_span = path.span;
        let path = Path::new(&path.item);

        let file = match (append, path.exists()) {
            (true, true) => std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(path),
            _ => std::fs::File::create(path),
        };

        let mut file = match file {
            Ok(file) => file,
            Err(err) => {
                return Ok(PipelineData::Value(
                    Value::Error {
                        error: ShellError::GenericError(
                            "Permission denied".into(),
                            err.to_string(),
                            Some(arg_span),
                            None,
                            Vec::new(),
                        ),
                    },
                    None,
                ));
            }
        };

        let ext = if raw {
            None
        } else {
            path.extension()
                .map(|name| name.to_string_lossy().to_string())
        };

        if let Some(ext) = ext {
            let output = match engine_state.find_decl(format!("to {}", ext).as_bytes(), &[]) {
                Some(converter_id) => {
                    let output = engine_state.get_decl(converter_id).run(
                        engine_state,
                        stack,
                        &Call::new(span),
                        input,
                    )?;

                    output.into_value(span)
                }
                None => input.into_value(span),
            };

            match output {
                Value::String { val, .. } => {
                    if let Err(err) = file.write_all(val.as_bytes()) {
                        return Err(ShellError::IOError(err.to_string()));
                    } else {
                        file.flush()?
                    }

                    Ok(PipelineData::new(span))
                }
                Value::Binary { val, .. } => {
                    if let Err(err) = file.write_all(&val) {
                        return Err(ShellError::IOError(err.to_string()));
                    } else {
                        file.flush()?
                    }

                    Ok(PipelineData::new(span))
                }
                Value::List { vals, .. } => {
                    let val = vals
                        .into_iter()
                        .map(|it| it.as_string())
                        .collect::<Result<Vec<String>, ShellError>>()?
                        .join("\n")
                        + "\n";

                    if let Err(err) = file.write_all(val.as_bytes()) {
                        return Err(ShellError::IOError(err.to_string()));
                    } else {
                        file.flush()?
                    }

                    Ok(PipelineData::new(span))
                }
                v => Err(ShellError::UnsupportedInput(
                    format!("{:?} not supported", v.get_type()),
                    span,
                )),
            }
        } else {
            match input {
                PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::new(span)),
                PipelineData::ExternalStream {
                    stdout: Some(mut stream),
                    ..
                } => {
                    let mut writer = BufWriter::new(file);

                    stream
                        .try_for_each(move |result| {
                            let buf = match result {
                                Ok(v) => match v {
                                    Value::String { val, .. } => val.into_bytes(),
                                    Value::Binary { val, .. } => val,
                                    _ => {
                                        return Err(ShellError::UnsupportedInput(
                                            format!("{:?} not supported", v.get_type()),
                                            v.span()?,
                                        ));
                                    }
                                },
                                Err(err) => return Err(err),
                            };

                            if let Err(err) = writer.write(&buf) {
                                return Err(ShellError::IOError(err.to_string()));
                            }
                            Ok(())
                        })
                        .map(|_| PipelineData::new(span))
                }
                input => match input.into_value(span) {
                    Value::String { val, .. } => {
                        if let Err(err) = file.write_all(val.as_bytes()) {
                            return Err(ShellError::IOError(err.to_string()));
                        } else {
                            file.flush()?
                        }

                        Ok(PipelineData::new(span))
                    }
                    Value::Binary { val, .. } => {
                        if let Err(err) = file.write_all(&val) {
                            return Err(ShellError::IOError(err.to_string()));
                        } else {
                            file.flush()?
                        }

                        Ok(PipelineData::new(span))
                    }
                    Value::List { vals, .. } => {
                        let val = vals
                            .into_iter()
                            .map(|it| it.as_string())
                            .collect::<Result<Vec<String>, ShellError>>()?
                            .join("\n")
                            + "\n";

                        if let Err(err) = file.write_all(val.as_bytes()) {
                            return Err(ShellError::IOError(err.to_string()));
                        } else {
                            file.flush()?
                        }

                        Ok(PipelineData::new(span))
                    }
                    v => Err(ShellError::UnsupportedInput(
                        format!("{:?} not supported", v.get_type()),
                        span,
                    )),
                },
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Save a string to foo.txt in the current directory",
                example: r#"echo 'save me' | save foo.txt"#,
                result: None,
            },
            Example {
                description: "Append a string to the end of foo.txt",
                example: r#"echo 'append me' | save --append foo.txt"#,
                result: None,
            },
            Example {
                description: "Save a record to foo.json in the current directory",
                example: r#"echo { a: 1, b: 2 } | save foo.json"#,
                result: None,
            },
        ]
    }
}
