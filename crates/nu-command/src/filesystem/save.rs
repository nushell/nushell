use nu_engine::current_dir;
use nu_engine::CallExt;
use nu_path::expand_path_with;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, DataSource, Example, PipelineData, PipelineMetadata, RawStream, ShellError,
    Signature, Span, Spanned, SyntaxShape, Type, Value,
};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;

use crate::progress_bar;

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
            .required("filename", SyntaxShape::Filepath, "The filename to use.")
            .named(
                "stderr",
                SyntaxShape::Filepath,
                "the filename used to save stderr, only works with `-r` flag",
                Some('e'),
            )
            .switch("raw", "save file as raw binary", Some('r'))
            .switch("append", "append input to the end of the file", Some('a'))
            .switch("force", "overwrite the destination", Some('f'))
            .switch("progress", "enable progress bar", Some('p'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let raw = call.has_flag(engine_state, stack, "raw")?;
        let append = call.has_flag(engine_state, stack, "append")?;
        let force = call.has_flag(engine_state, stack, "force")?;
        let progress = call.has_flag(engine_state, stack, "progress")?;
        let out_append = if let Some(Expression {
            expr: Expr::Bool(out_append),
            ..
        }) = call.get_parser_info("out-append")
        {
            *out_append
        } else {
            false
        };
        let err_append = if let Some(Expression {
            expr: Expr::Bool(err_append),
            ..
        }) = call.get_parser_info("err-append")
        {
            *err_append
        } else {
            false
        };

        let span = call.head;
        let cwd = current_dir(engine_state, stack)?;

        let path_arg = call.req::<Spanned<PathBuf>>(engine_state, stack, 0)?;
        let path = Spanned {
            item: expand_path_with(path_arg.item, &cwd),
            span: path_arg.span,
        };

        let stderr_path = call
            .get_flag::<Spanned<PathBuf>>(engine_state, stack, "stderr")?
            .map(|arg| Spanned {
                item: expand_path_with(arg.item, cwd),
                span: arg.span,
            });

        match input {
            PipelineData::ExternalStream { stdout: None, .. } => {
                // Open files to possibly truncate them
                let _ = get_files(&path, stderr_path.as_ref(), append, false, false, force)?;
                Ok(PipelineData::empty())
            }
            PipelineData::ExternalStream {
                stdout: Some(stream),
                stderr,
                ..
            } => {
                let (file, stderr_file) = get_files(
                    &path,
                    stderr_path.as_ref(),
                    append,
                    out_append,
                    err_append,
                    force,
                )?;

                // delegate a thread to redirect stderr to result.
                let handler = stderr.map(|stderr_stream| match stderr_file {
                    Some(stderr_file) => thread::Builder::new()
                        .name("stderr redirector".to_string())
                        .spawn(move || stream_to_file(stderr_stream, stderr_file, span, progress))
                        .expect("Failed to create thread"),
                    None => thread::Builder::new()
                        .name("stderr redirector".to_string())
                        .spawn(move || {
                            let _ = stderr_stream.into_bytes();
                            Ok(PipelineData::empty())
                        })
                        .expect("Failed to create thread"),
                });

                let res = stream_to_file(stream, file, span, progress);
                if let Some(h) = handler {
                    h.join().map_err(|err| ShellError::ExternalCommand {
                        label: "Fail to receive external commands stderr message".to_string(),
                        help: format!("{err:?}"),
                        span,
                    })??;
                    res
                } else {
                    res
                }
            }
            PipelineData::ListStream(ls, pipeline_metadata)
                if raw || prepare_path(&path, append, force)?.0.extension().is_none() =>
            {
                if let Some(PipelineMetadata {
                    data_source: DataSource::FilePath(input_path),
                }) = pipeline_metadata
                {
                    if path.item == input_path {
                        return Err(ShellError::GenericError {
                            error: "pipeline input and output are same file".into(),
                            msg: format!(
                                "can't save output to '{}' while it's being reading",
                                path.item.display()
                            ),
                            span: Some(path.span),
                            help: Some("you should change output path".into()),
                            inner: vec![],
                        });
                    }

                    if let Some(ref err_path) = stderr_path {
                        if err_path.item == input_path {
                            return Err(ShellError::GenericError {
                                error: "pipeline input and stderr are same file".into(),
                                msg: format!(
                                    "can't save stderr to '{}' while it's being reading",
                                    err_path.item.display()
                                ),
                                span: Some(err_path.span),
                                help: Some("you should change stderr path".into()),
                                inner: vec![],
                            });
                        }
                    }
                }

                let (mut file, _) = get_files(
                    &path,
                    stderr_path.as_ref(),
                    append,
                    out_append,
                    err_append,
                    force,
                )?;
                for val in ls {
                    file.write_all(&value_to_bytes(val)?)
                        .map_err(|err| ShellError::IOError {
                            msg: err.to_string(),
                        })?;
                    file.write_all("\n".as_bytes())
                        .map_err(|err| ShellError::IOError {
                            msg: err.to_string(),
                        })?;
                }
                file.flush()?;

                Ok(PipelineData::empty())
            }
            input => {
                let bytes =
                    input_to_bytes(input, Path::new(&path.item), raw, engine_state, stack, span)?;

                // Only open file after successful conversion
                let (mut file, _) = get_files(
                    &path,
                    stderr_path.as_ref(),
                    append,
                    out_append,
                    err_append,
                    force,
                )?;

                file.write_all(&bytes).map_err(|err| ShellError::IOError {
                    msg: err.to_string(),
                })?;

                file.flush()?;

                Ok(PipelineData::empty())
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

/// Convert [`PipelineData`] bytes to write in file, possibly converting
/// to format of output file
fn input_to_bytes(
    input: PipelineData,
    path: &Path,
    raw: bool,
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
) -> Result<Vec<u8>, ShellError> {
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
        convert_to_extension(engine_state, &ext, stack, input, span)
    } else {
        let value = input.into_value(span);
        value_to_bytes(value)
    }
}

/// Convert given data into content of file of specified extension if
/// corresponding `to` command exists. Otherwise attempt to convert
/// data to bytes as is
fn convert_to_extension(
    engine_state: &EngineState,
    extension: &str,
    stack: &mut Stack,
    input: PipelineData,
    span: Span,
) -> Result<Vec<u8>, ShellError> {
    let converter = engine_state.find_decl(format!("to {extension}").as_bytes(), &[]);

    let output = match converter {
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

    value_to_bytes(output)
}

/// Convert [`Value::String`] [`Value::Binary`] or [`Value::List`] into [`Vec`] of bytes
///
/// Propagates [`Value::Error`] and creates error otherwise
fn value_to_bytes(value: Value) -> Result<Vec<u8>, ShellError> {
    match value {
        Value::String { val, .. } => Ok(val.into_bytes()),
        Value::Binary { val, .. } => Ok(val),
        Value::List { vals, .. } => {
            let val = vals
                .into_iter()
                .map(Value::coerce_into_string)
                .collect::<Result<Vec<String>, ShellError>>()?
                .join("\n")
                + "\n";

            Ok(val.into_bytes())
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { error, .. } => Err(*error),
        other => Ok(other.coerce_into_string()?.into_bytes()),
    }
}

/// Convert string path to [`Path`] and [`Span`] and check if this path
/// can be used with given flags
fn prepare_path(
    path: &Spanned<PathBuf>,
    append: bool,
    force: bool,
) -> Result<(&Path, Span), ShellError> {
    let span = path.span;
    let path = &path.item;

    if !(force || append) && path.exists() {
        Err(ShellError::GenericError {
            error: "Destination file already exists".into(),
            msg: format!(
                "Destination file '{}' already exists",
                path.to_string_lossy()
            ),
            span: Some(span),
            help: Some("you can use -f, --force to force overwriting the destination".into()),
            inner: vec![],
        })
    } else {
        Ok((path, span))
    }
}

fn open_file(path: &Path, span: Span, append: bool) -> Result<File, ShellError> {
    let file = match (append, path.exists()) {
        (true, true) => std::fs::OpenOptions::new().append(true).open(path),
        _ => std::fs::File::create(path),
    };

    file.map_err(|e| ShellError::GenericError {
        error: "Permission denied".into(),
        msg: e.to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    })
}

/// Get output file and optional stderr file
fn get_files(
    path: &Spanned<PathBuf>,
    stderr_path: Option<&Spanned<PathBuf>>,
    append: bool,
    out_append: bool,
    err_append: bool,
    force: bool,
) -> Result<(File, Option<File>), ShellError> {
    // First check both paths
    let (path, path_span) = prepare_path(path, append || out_append, force)?;
    let stderr_path_and_span = stderr_path
        .as_ref()
        .map(|stderr_path| prepare_path(stderr_path, append || err_append, force))
        .transpose()?;

    // Only if both files can be used open and possibly truncate them
    let file = open_file(path, path_span, append || out_append)?;

    let stderr_file = stderr_path_and_span
        .map(|(stderr_path, stderr_path_span)| {
            if path == stderr_path {
                Err(ShellError::GenericError {
                    error: "input and stderr input to same file".into(),
                    msg: "can't save both input and stderr input to the same file".into(),
                    span: Some(stderr_path_span),
                    help: Some("you should use `o+e> file` instead".into()),
                    inner: vec![],
                })
            } else {
                open_file(stderr_path, stderr_path_span, append || err_append)
            }
        })
        .transpose()?;

    Ok((file, stderr_file))
}

fn stream_to_file(
    mut stream: RawStream,
    file: File,
    span: Span,
    progress: bool,
) -> Result<PipelineData, ShellError> {
    // https://github.com/nushell/nushell/pull/9377 contains the reason
    // for not using BufWriter<File>
    let mut writer = file;

    let mut bytes_processed: u64 = 0;
    let bytes_processed_p = &mut bytes_processed;
    let file_total_size = stream.known_size;
    let mut process_failed = false;
    let process_failed_p = &mut process_failed;

    // Create the progress bar
    // It looks a bit messy but I am doing it this way to avoid
    // creating the bar when is not needed
    let (mut bar_opt, bar_opt_clone) = if progress {
        let tmp_bar = progress_bar::NuProgressBar::new(file_total_size);
        let tmp_bar_clone = tmp_bar.clone();

        (Some(tmp_bar), Some(tmp_bar_clone))
    } else {
        (None, None)
    };

    let result = stream
        .try_for_each(move |result| {
            let buf = match result {
                Ok(v) => match v {
                    Value::String { val, .. } => val.into_bytes(),
                    Value::Binary { val, .. } => val,
                    // Propagate errors by explicitly matching them before the final case.
                    Value::Error { error, .. } => return Err(*error),
                    other => {
                        return Err(ShellError::OnlySupportsThisInputType {
                            exp_input_type: "string or binary".into(),
                            wrong_type: other.get_type().to_string(),
                            dst_span: span,
                            src_span: other.span(),
                        });
                    }
                },
                Err(err) => {
                    *process_failed_p = true;
                    return Err(err);
                }
            };

            // If the `progress` flag is set then
            if progress {
                // Update the total amount of bytes that has been saved and then print the progress bar
                *bytes_processed_p += buf.len() as u64;
                if let Some(bar) = &mut bar_opt {
                    bar.update_bar(*bytes_processed_p);
                }
            }

            if let Err(err) = writer.write(&buf) {
                *process_failed_p = true;
                return Err(ShellError::IOError {
                    msg: err.to_string(),
                });
            }
            Ok(())
        })
        .map(|_| PipelineData::empty());

    // If the `progress` flag is set then
    if progress {
        // If the process failed, stop the progress bar with an error message.
        if process_failed {
            if let Some(bar) = bar_opt_clone {
                bar.abandoned_msg("# Error while saving #".to_owned());
            }
        }
    }

    // And finally return the stream result.
    result
}
