use crate::progress_bar;
use nu_engine::get_eval_block;
#[allow(deprecated)]
use nu_engine::{command_prelude::*, current_dir};
use nu_path::expand_path_with;
use nu_protocol::{
    ByteStreamSource, DataSource, OutDest, PipelineMetadata, Signals, ast,
    byte_stream::copy_with_signals, process::ChildPipe, shell_error::io::IoError,
};
use std::{
    borrow::Cow,
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Save;

impl Command for Save {
    fn name(&self) -> &str {
        "save"
    }

    fn description(&self) -> &str {
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

        let span = call.head;
        #[allow(deprecated)]
        let cwd = current_dir(engine_state, stack)?;

        let path_arg = call.req::<Spanned<PathBuf>>(engine_state, stack, 0)?;
        let path = Spanned {
            item: expand_path_with(path_arg.item, &cwd, true),
            span: path_arg.span,
        };

        let stderr_path = call
            .get_flag::<Spanned<PathBuf>>(engine_state, stack, "stderr")?
            .map(|arg| Spanned {
                item: expand_path_with(arg.item, cwd, true),
                span: arg.span,
            });

        let from_io_error = IoError::factory(span, path.item.as_path());
        match input {
            PipelineData::ByteStream(stream, metadata) => {
                check_saving_to_source_file(metadata.as_ref(), &path, stderr_path.as_ref())?;

                let (file, stderr_file) =
                    get_files(engine_state, &path, stderr_path.as_ref(), append, force)?;

                let size = stream.known_size();
                let signals = engine_state.signals();

                match stream.into_source() {
                    ByteStreamSource::Read(read) => {
                        stream_to_file(read, size, signals, file, span, progress)?;
                    }
                    ByteStreamSource::File(source) => {
                        stream_to_file(source, size, signals, file, span, progress)?;
                    }
                    #[cfg(feature = "os")]
                    ByteStreamSource::Child(mut child) => {
                        fn write_or_consume_stderr(
                            stderr: ChildPipe,
                            file: Option<File>,
                            span: Span,
                            signals: &Signals,
                            progress: bool,
                        ) -> Result<(), ShellError> {
                            if let Some(file) = file {
                                match stderr {
                                    ChildPipe::Pipe(pipe) => {
                                        stream_to_file(pipe, None, signals, file, span, progress)
                                    }
                                    ChildPipe::Tee(tee) => {
                                        stream_to_file(tee, None, signals, file, span, progress)
                                    }
                                }?
                            } else {
                                match stderr {
                                    ChildPipe::Pipe(mut pipe) => {
                                        io::copy(&mut pipe, &mut io::stderr())
                                    }
                                    ChildPipe::Tee(mut tee) => {
                                        io::copy(&mut tee, &mut io::stderr())
                                    }
                                }
                                .map_err(|err| IoError::new(err, span, None))?;
                            }
                            Ok(())
                        }

                        match (child.stdout.take(), child.stderr.take()) {
                            (Some(stdout), stderr) => {
                                // delegate a thread to redirect stderr to result.
                                let handler = stderr
                                    .map(|stderr| {
                                        let signals = signals.clone();
                                        thread::Builder::new().name("stderr saver".into()).spawn(
                                            move || {
                                                write_or_consume_stderr(
                                                    stderr,
                                                    stderr_file,
                                                    span,
                                                    &signals,
                                                    progress,
                                                )
                                            },
                                        )
                                    })
                                    .transpose()
                                    .map_err(&from_io_error)?;

                                let res = match stdout {
                                    ChildPipe::Pipe(pipe) => {
                                        stream_to_file(pipe, None, signals, file, span, progress)
                                    }
                                    ChildPipe::Tee(tee) => {
                                        stream_to_file(tee, None, signals, file, span, progress)
                                    }
                                };
                                if let Some(h) = handler {
                                    h.join().map_err(|err| ShellError::ExternalCommand {
                                        label: "Fail to receive external commands stderr message"
                                            .to_string(),
                                        help: format!("{err:?}"),
                                        span,
                                    })??;
                                }
                                res?;
                            }
                            (None, Some(stderr)) => {
                                write_or_consume_stderr(
                                    stderr,
                                    stderr_file,
                                    span,
                                    signals,
                                    progress,
                                )?;
                            }
                            (None, None) => {}
                        };

                        child.wait()?;
                    }
                }

                Ok(PipelineData::empty())
            }
            PipelineData::ListStream(ls, pipeline_metadata)
                if raw || prepare_path(&path, append, force)?.0.extension().is_none() =>
            {
                check_saving_to_source_file(
                    pipeline_metadata.as_ref(),
                    &path,
                    stderr_path.as_ref(),
                )?;

                let (mut file, _) =
                    get_files(engine_state, &path, stderr_path.as_ref(), append, force)?;
                for val in ls {
                    file.write_all(&value_to_bytes(val)?)
                        .map_err(&from_io_error)?;
                    file.write_all("\n".as_bytes()).map_err(&from_io_error)?;
                }
                file.flush().map_err(&from_io_error)?;

                Ok(PipelineData::empty())
            }
            input => {
                // It's not necessary to check if we are saving to the same file if this is a
                // collected value, and not a stream
                if !matches!(input, PipelineData::Value(..) | PipelineData::Empty) {
                    check_saving_to_source_file(
                        input.metadata().as_ref(),
                        &path,
                        stderr_path.as_ref(),
                    )?;
                }

                // Try to convert the input pipeline into another type if we know the extension
                let ext = extract_extension(&input, &path.item, raw);
                let converted = match ext {
                    None => input,
                    Some(ext) => convert_to_extension(engine_state, &ext, stack, input, span)?,
                };

                // Save custom value however they implement saving
                if let PipelineData::Value(Value::Custom { val, internal_span }, ..) = converted {
                    return val
                        .save(
                            Spanned {
                                item: &path.item,
                                span: path.span,
                            },
                            internal_span,
                            span,
                        )
                        .map(|()| PipelineData::empty());
                }

                let bytes = value_to_bytes(converted.into_value(span)?)?;

                // Only open file after successful conversion
                let (mut file, _) =
                    get_files(engine_state, &path, stderr_path.as_ref(), append, force)?;

                file.write_all(&bytes).map_err(&from_io_error)?;
                file.flush().map_err(&from_io_error)?;

                Ok(PipelineData::empty())
            }
        }
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
            Example {
                description: "Show the extensions for which the `save` command will automatically serialize",
                example: r#"scope commands
    | where name starts-with "to "
    | insert extension { get name | str replace -r "^to " "" | $"*.($in)" }
    | select extension name
    | rename extension command
"#,
                result: None,
            },
        ]
    }

    fn pipe_redirection(&self) -> (Option<OutDest>, Option<OutDest>) {
        (Some(OutDest::PipeSeparate), Some(OutDest::PipeSeparate))
    }
}

fn saving_to_source_file_error(dest: &Spanned<PathBuf>) -> ShellError {
    ShellError::GenericError {
        error: "pipeline input and output are the same file".into(),
        msg: format!(
            "can't save output to '{}' while it's being read",
            dest.item.display()
        ),
        span: Some(dest.span),
        help: Some(
            "insert a `collect` command in the pipeline before `save` (see `help collect`).".into(),
        ),
        inner: vec![],
    }
}

fn check_saving_to_source_file(
    metadata: Option<&PipelineMetadata>,
    dest: &Spanned<PathBuf>,
    stderr_dest: Option<&Spanned<PathBuf>>,
) -> Result<(), ShellError> {
    let Some(DataSource::FilePath(source)) = metadata.map(|meta| &meta.data_source) else {
        return Ok(());
    };

    if &dest.item == source {
        return Err(saving_to_source_file_error(dest));
    }

    if let Some(dest) = stderr_dest
        && &dest.item == source
    {
        return Err(saving_to_source_file_error(dest));
    }

    Ok(())
}

/// Extract extension for conversion.
fn extract_extension<'e>(input: &PipelineData, path: &'e Path, raw: bool) -> Option<Cow<'e, str>> {
    match (raw, input) {
        (true, _)
        | (_, PipelineData::ByteStream(..))
        | (_, PipelineData::Value(Value::String { .. }, ..)) => None,
        _ => path.extension().map(|name| name.to_string_lossy()),
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
) -> Result<PipelineData, ShellError> {
    if let Some(decl_id) = engine_state.find_decl(format!("to {extension}").as_bytes(), &[]) {
        let decl = engine_state.get_decl(decl_id);
        if let Some(block_id) = decl.block_id() {
            let block = engine_state.get_block(block_id);
            let eval_block = get_eval_block(engine_state);
            eval_block(engine_state, stack, block, input).map(|p| p.body)
        } else {
            let call = ast::Call::new(span);
            decl.run(engine_state, stack, &(&call).into(), input)
        }
    } else {
        Ok(input)
    }
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

fn open_file(
    engine_state: &EngineState,
    path: &Path,
    span: Span,
    append: bool,
) -> Result<File, ShellError> {
    let file: std::io::Result<File> = match (append, path.exists()) {
        (true, true) => std::fs::OpenOptions::new().append(true).open(path),
        _ => {
            // This is a temporary solution until `std::fs::File::create` is fixed on Windows (rust-lang/rust#134893)
            // A TOCTOU problem exists here, which may cause wrong error message to be shown
            #[cfg(target_os = "windows")]
            if path.is_dir() {
                #[allow(
                    deprecated,
                    reason = "we don't get a IsADirectory error, so we need to provide it"
                )]
                Err(std::io::ErrorKind::IsADirectory.into())
            } else {
                std::fs::File::create(path)
            }
            #[cfg(not(target_os = "windows"))]
            std::fs::File::create(path)
        }
    };

    match file {
        Ok(file) => Ok(file),
        Err(err) => {
            // In caase of NotFound, search for the missing parent directory.
            // This also presents a TOCTOU (or TOUTOC, technically?)
            if err.kind() == std::io::ErrorKind::NotFound
                && let Some(missing_component) =
                    path.ancestors().skip(1).filter(|dir| !dir.exists()).last()
            {
                // By looking at the postfix to remove, rather than the prefix
                // to keep, we are able to handle relative paths too.
                let components_to_remove = path
                    .strip_prefix(missing_component)
                    .expect("Stripping ancestor from a path should never fail")
                    .as_os_str()
                    .as_encoded_bytes();

                return Err(ShellError::Io(IoError::new(
                    ErrorKind::DirectoryNotFound,
                    engine_state
                        .span_match_postfix(span, components_to_remove)
                        .map(|(pre, _post)| pre)
                        .unwrap_or(span),
                    PathBuf::from(missing_component),
                )));
            }

            Err(ShellError::Io(IoError::new(err, span, PathBuf::from(path))))
        }
    }
}

/// Get output file and optional stderr file
fn get_files(
    engine_state: &EngineState,
    path: &Spanned<PathBuf>,
    stderr_path: Option<&Spanned<PathBuf>>,
    append: bool,
    force: bool,
) -> Result<(File, Option<File>), ShellError> {
    // First check both paths
    let (path, path_span) = prepare_path(path, append, force)?;
    let stderr_path_and_span = stderr_path
        .as_ref()
        .map(|stderr_path| prepare_path(stderr_path, append, force))
        .transpose()?;

    // Only if both files can be used open and possibly truncate them
    let file = open_file(engine_state, path, path_span, append)?;

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
                open_file(engine_state, stderr_path, stderr_path_span, append)
            }
        })
        .transpose()?;

    Ok((file, stderr_file))
}

fn stream_to_file(
    source: impl Read,
    known_size: Option<u64>,
    signals: &Signals,
    mut file: File,
    span: Span,
    progress: bool,
) -> Result<(), ShellError> {
    // TODO: maybe we can get a path in here
    let from_io_error = IoError::factory(span, None);

    // https://github.com/nushell/nushell/pull/9377 contains the reason for not using `BufWriter`
    if progress {
        let mut bytes_processed = 0;

        let mut bar = progress_bar::NuProgressBar::new(known_size);

        let mut last_update = Instant::now();

        let mut reader = BufReader::new(source);

        let res = loop {
            if let Err(err) = signals.check(&span) {
                bar.abandoned_msg("# Cancelled #".to_owned());
                return Err(err);
            }

            match reader.fill_buf() {
                Ok(&[]) => break Ok(()),
                Ok(buf) => {
                    file.write_all(buf).map_err(&from_io_error)?;
                    let len = buf.len();
                    reader.consume(len);
                    bytes_processed += len as u64;
                    if last_update.elapsed() >= Duration::from_millis(75) {
                        bar.update_bar(bytes_processed);
                        last_update = Instant::now();
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => break Err(e),
            }
        };

        // If the process failed, stop the progress bar with an error message.
        if let Err(err) = res {
            let _ = file.flush();
            bar.abandoned_msg("# Error while saving #".to_owned());
            Err(from_io_error(err).into())
        } else {
            file.flush().map_err(&from_io_error)?;
            Ok(())
        }
    } else {
        copy_with_signals(source, file, span, signals)?;
        Ok(())
    }
}
