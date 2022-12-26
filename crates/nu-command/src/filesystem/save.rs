use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, RawStream, ShellError, Signature, Span, Spanned, SyntaxShape,
    Type, Value,
};
use std::fs::File;
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

        let span = call.head;

        let path = call.req::<Spanned<String>>(engine_state, stack, 0)?;
        let stderr_path = call.get_flag::<Spanned<String>>(engine_state, stack, "stderr")?;

        match input {
            PipelineData::ExternalStream { stdout: None, .. } => {
                // Open files to possibly truncate them
                let _ = get_files(&path, &stderr_path, append, force)?;
                Ok(PipelineData::empty())
            }
            PipelineData::ExternalStream {
                stdout: Some(stream),
                stderr,
                ..
            } => {
                let (file, stderr_file) = get_files(&path, &stderr_path, append, force)?;

                // delegate a thread to redirect stderr to result.
                let handler = stderr.map(|stderr_stream| match stderr_file {
                    Some(stderr_file) => {
                        std::thread::spawn(move || stream_to_file(stderr_stream, stderr_file, span))
                    }
                    None => std::thread::spawn(move || {
                        let _ = stderr_stream.into_bytes();
                        Ok(PipelineData::empty())
                    }),
                });

                let res = stream_to_file(stream, file, span);
                if let Some(h) = handler {
                    h.join().map_err(|err| {
                        ShellError::ExternalCommand(
                            "Fail to receive external commands stderr message".to_string(),
                            format!("{err:?}"),
                            span,
                        )
                    })??;
                    res
                } else {
                    res
                }
            }
            input => {
                let bytes = input_to_bytes(
                    input,
                    &Path::new(&path.item),
                    raw,
                    engine_state,
                    stack,
                    span,
                )?;

                // Only open file after successful conversion
                let (mut file, _) = get_files(&path, &stderr_path, append, force)?;

                file.write_all(&bytes)
                    .map_err(|err| ShellError::IOError(err.to_string()))?;

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
        string_binary_list_value_to_bytes(value, span)
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
    let converter = engine_state.find_decl(format!("to {}", extension).as_bytes(), &[]);

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

    string_binary_list_value_to_bytes(output, span)
}

/// Convert [`Value::String`] [`Value::Binary`] or [`Value::List`] into [`Vec`] of bytes
///
/// Propagates [`Value::Error`] and creates error otherwise
fn string_binary_list_value_to_bytes(value: Value, span: Span) -> Result<Vec<u8>, ShellError> {
    match value {
        Value::String { val, .. } => Ok(val.into_bytes()),
        Value::Binary { val, .. } => Ok(val),
        Value::List { vals, .. } => {
            let val = vals
                .into_iter()
                .map(|it| it.as_string())
                .collect::<Result<Vec<String>, ShellError>>()?
                .join("\n")
                + "\n";

            Ok(val.into_bytes())
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
}

/// Convert string path to [`Path`] and [`Span`] and check if this path
/// can be used with given flags
fn prepare_path<'a>(
    path: &'a Spanned<String>,
    append: bool,
    force: bool,
) -> Result<(&'a Path, Span), ShellError> {
    let span = path.span;
    let path = Path::new(&path.item);

    if !(force || append) && path.exists() {
        Err(ShellError::GenericError(
            "Destination file already exists".into(),
            format!(
                "Destination file '{}' already exists",
                path.to_string_lossy()
            ),
            Some(span),
            Some("you can use -f, --force to force overwriting the destination".into()),
            Vec::new(),
        ))
    } else {
        Ok((path, span))
    }
}

fn open_file(path: &Path, span: Span, append: bool) -> Result<File, ShellError> {
    let file = match (append, path.exists()) {
        (true, true) => std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(path),
        _ => std::fs::File::create(path),
    };

    file.map_err(|err| {
        ShellError::GenericError(
            "Permission denied".into(),
            err.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })
}

fn clone_file(file: &File, span: Span) -> Result<File, ShellError> {
    file.try_clone().map_err(|err| {
        ShellError::GenericError(
            "Permission denied".into(),
            err.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })
}

/// Get output file and optional stderr file
fn get_files(
    path: &Spanned<String>,
    stderr_path: &Option<Spanned<String>>,
    append: bool,
    force: bool,
) -> Result<(File, Option<File>), ShellError> {
    // First check both paths
    let (path, path_span) = prepare_path(path, append, force)?;
    let stderr_path_and_span = stderr_path
        .as_ref()
        .map(|stderr_path| prepare_path(&stderr_path, append, force))
        .transpose()?;

    // Only if both files can be used open and possibly truncate them
    let file = open_file(path, path_span, append)?;

    let stderr_file = stderr_path_and_span
        .map(|(stderr_path, stderr_path_span)| {
            if path == stderr_path {
                clone_file(&file, stderr_path_span)
            } else {
                open_file(stderr_path, stderr_path_span, append)
            }
        })
        .transpose()?;

    Ok((file, stderr_file))
}

fn stream_to_file(
    mut stream: RawStream,
    file: File,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let mut writer = BufWriter::new(file);

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

            if let Err(err) = writer.write(&buf) {
                return Err(ShellError::IOError(err.to_string()));
            }
            Ok(())
        })
        .map(|_| PipelineData::empty())
}
