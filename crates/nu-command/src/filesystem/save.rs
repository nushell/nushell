use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, RawStream, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};
use std::fs::File;
use std::io::{BufWriter, Write, stdout};
use std::path::Path;

use crossterm::{
    QueueableCommand, style::Print,
    cursor
};

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
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .required("filename", SyntaxShape::Filepath, "the filename to use")
            .named(
                "stderr",
                SyntaxShape::Filepath,
                "the filename used to save stderr, only works with `-r` flag",
                Some('e'),
            )
            .switch("raw", "save file as raw binary", Some('r'))
            .switch("append", "append input to the end of the file", Some('a'))
            .switch("force", "overwrite the destination", Some('f'))
            .switch("progress", "overwrite the destination", Some('p'))
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
        let force = call.has_flag("force");
        let progress = call.has_flag("progress");

        let span = call.head;

        let path = call.req::<Spanned<String>>(engine_state, stack, 0)?;
        let arg_span = path.span;
        let path = Path::new(&path.item);

        let path_exists = path.exists();
        if path_exists && !force && !append {
            return Err(ShellError::GenericError(
                "Destination file already exists".into(),
                format!(
                    "Destination file '{}' already exists",
                    path.to_string_lossy()
                ),
                Some(arg_span),
                Some("you can use -f, --force to force overwriting the destination".into()),
                Vec::new(),
            ));
        }

        let file = match (append, path_exists) {
            (true, true) => std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(path),
            _ => std::fs::File::create(path),
        };

        let mut file = match file {
            Ok(file) => file,
            Err(err) => {
                return Err(ShellError::GenericError(
                    "Permission denied".into(),
                    err.to_string(),
                    Some(arg_span),
                    None,
                    Vec::new(),
                ));
            }
        };
        let stderr_path = call.get_flag::<Spanned<String>>(engine_state, stack, "stderr")?;
        let stderr_file = match stderr_path {
            None => None,
            Some(stderr_path) => {
                let stderr_span = stderr_path.span;
                let stderr_path = Path::new(&stderr_path.item);
                if stderr_path == path {
                    Some(file.try_clone()?)
                } else {
                    match std::fs::File::create(stderr_path) {
                        Ok(file) => Some(file),
                        Err(err) => {
                            return Err(ShellError::GenericError(
                                "Permission denied".into(),
                                err.to_string(),
                                Some(stderr_span),
                                None,
                                Vec::new(),
                            ))
                        }
                    }
                }
            }
        };

        let ext = if raw {
            None
        // if is extern stream , in other words , not value
        } else if let PipelineData::ExternalStream { .. } = input {
            None
        } else if let PipelineData::Value(Value::String { .. }, ..) = input {
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

                    Ok(PipelineData::empty())
                }
                Value::Binary { val, .. } => {
                    if let Err(err) = file.write_all(&val) {
                        return Err(ShellError::IOError(err.to_string()));
                    } else {
                        file.flush()?
                    }

                    Ok(PipelineData::empty())
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

                    Ok(PipelineData::empty())
                }
                // Propagate errors by explicitly matching them before the final case.
                Value::Error { error } => Err(error),
                other => Err(ShellError::OnlySupportsThisInputType(
                    "string, binary or list".into(),
                    other.get_type().to_string(),
                    span,
                    // This line requires the Value::Error match above.
                    other.expect_span(),
                )),
            }
        } else {
            match input {
                PipelineData::ExternalStream { stdout: None, .. } => Ok(PipelineData::empty()),
                PipelineData::ExternalStream {
                    stdout: Some(stream),
                    stderr,
                    ..
                } => {
                    // delegate a thread to redirect stderr to result.
                    let handler = stderr.map(|stderr_stream| match stderr_file {
                        Some(stderr_file) => std::thread::spawn(move || {
                            stream_to_file(stderr_stream, stderr_file, span, progress)
                        }),
                        None => std::thread::spawn(move || {
                            let _ = stderr_stream.into_bytes();
                            Ok(PipelineData::empty())
                        }),
                    });

                    let res = stream_to_file(stream, file, span, progress);
                    if let Some(h) = handler {
                        match h.join() {
                            Err(err) => {
                                return Err(ShellError::ExternalCommand(
                                    "Fail to receive external commands stderr message".to_string(),
                                    format!("{err:?}"),
                                    span,
                                ))
                            }
                            Ok(res) => res,
                        }?;
                        res
                    } else {
                        res
                    }
                }
                input => match input.into_value(span) {
                    Value::String { val, .. } => {
                        if let Err(err) = file.write_all(val.as_bytes()) {
                            return Err(ShellError::IOError(err.to_string()));
                        } else {
                            file.flush()?
                        }

                        Ok(PipelineData::empty())
                    }
                    Value::Binary { val, .. } => {
                        if let Err(err) = file.write_all(&val) {
                            return Err(ShellError::IOError(err.to_string()));
                        } else {
                            file.flush()?
                        }

                        Ok(PipelineData::empty())
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

                        Ok(PipelineData::empty())
                    }
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { error } => Err(error),
                    other => Err(ShellError::OnlySupportsThisInputType(
                        "string, binary or list".into(),
                        other.get_type().to_string(),
                        span,
                        // This line requires the Value::Error match above.
                        other.expect_span(),
                    )),
                },
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Save a string to foo.txt in the current directory",
                example: r#"'save me' | save foo.txt"#,
                result: None,
            },
            Example {
                description: "Append a string to the end of foo.txt",
                example: r#"'append me' | save --append foo.txt"#,
                result: None,
            },
            Example {
                description: "Save a record to foo.json in the current directory",
                example: r#"{ a: 1, b: 2 } | save foo.json"#,
                result: None,
            },
            Example {
                description: "Save a running program's stderr to foo.txt",
                example: r#"do -i {} | save foo.txt --stderr foo.txt"#,
                result: None,
            },
            Example {
                description: "Save a running program's stderr to separate file",
                example: r#"do -i {} | save foo.txt --stderr bar.txt"#,
                result: None,
            },
        ]
    }
}

fn stream_to_file(
    mut stream: RawStream,
    file: File,
    span: Span,
    progress: bool
) -> Result<PipelineData, ShellError> {
    let mut writer = BufWriter::new(file);

    let mut counter = 0.0;
    let cc = &mut counter;
    let test = std::mem::size_of_val(&stream);
    println!("test: {:?}", test);
    println!("[                    ] ");
    let cursor_pos = cursor::position().unwrap();
    let max_cursor = 20;
    let cursor_dir = 1;
    let mut current_x_cursor: u16 = 0;

    println!("cursor pos: {:?}", cursor_pos);

    stream
        .try_for_each(move |result| {
            let buf = match result {
                Ok(v) => match v {
                    Value::String { val, .. } => val.into_bytes(),
                    Value::Binary { val, .. } => val,
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { error } => return Err(error),
                    other => {
                        return Err(ShellError::OnlySupportsThisInputType(
                            "string or binary".into(),
                            other.get_type().to_string(),
                            span,
                            // This line requires the Value::Error match above.
                            other.expect_span(),
                        ));
                    }
                },
                Err(err) => return Err(err),
            };

            current_x_cursor += 1;
            if current_x_cursor >= max_cursor { current_x_cursor = 0 };

            let tmp_bar1 = " ".repeat(current_x_cursor as usize)+"#";
            let tmp_bar2 = " ".repeat((max_cursor - current_x_cursor - 1) as usize);
            let tmp_bar1 = format!("{}{}", tmp_bar1, tmp_bar2);

            if progress{
                *cc +=  buf.len() as f64 / 1000000.0;
                let amount_downloaded = format!("{:.2}MB", *cc);
    
                stdout()
                    .queue(cursor::SavePosition)
                    .unwrap()
                    .queue(cursor::MoveTo(cursor_pos.0+1, cursor_pos.1-2))
                    .unwrap()
                    .queue(Print(tmp_bar1))
                    .unwrap()
                    .queue(cursor::MoveTo(cursor_pos.0+30, cursor_pos.1-2))
                    .unwrap()
                    .queue(Print(amount_downloaded))
                    .unwrap()
                    .queue(cursor::RestorePosition)
                    .unwrap();
    
                stdout().flush().unwrap();
                //println!("{}", *cc);
            }

            if let Err(err) = writer.write(&buf) {
                return Err(ShellError::IOError(err.to_string()));
            }
            Ok(())
        })
        .map(|_| PipelineData::empty())
}
