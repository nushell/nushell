use std::fs::read_link;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use nu_cmd_base::arg_glob;
use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};

use super::util::try_interaction;

use crate::filesystem::util::FileStructure;
use crate::progress_bar;

#[derive(Clone)]
pub struct Cp;

impl Command for Cp {
    fn name(&self) -> &str {
        "cp-old"
    }

    fn usage(&self) -> &str {
        "Old nushell version of Copy files."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp-old")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("source", SyntaxShape::GlobPattern, "The place to copy from.")
            .required("destination", SyntaxShape::Filepath, "The place to copy to.")
            .switch(
                "recursive",
                "copy recursively through subdirectories",
                Some('r'),
            )
            .switch(
                "verbose",
                "show successful copies in addition to failed copies (default:false)",
                Some('v'),
            )
            .switch("update",
                "copy only when the SOURCE file is newer than the destination file or when the destination file is missing",
                Some('u')
            )
            // TODO: add back in additional features
            // .switch("force", "suppress error when no file", Some('f'))
            .switch("interactive", "ask user to confirm action", Some('i'))
            .switch(
                "no-symlink",
                "no symbolic links are followed, only works if -r is active",
                Some('n'),
            )
            .switch("progress", "enable progress bar", Some('p'))
            .category(Category::FileSystem)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let src: Spanned<String> = call.req(engine_state, stack, 0)?;
        let dst: Spanned<String> = call.req(engine_state, stack, 1)?;
        let recursive = call.has_flag("recursive");
        let verbose = call.has_flag("verbose");
        let interactive = call.has_flag("interactive");
        let progress = call.has_flag("progress");
        let update_mode = call.has_flag("update");

        let current_dir_path = current_dir(engine_state, stack)?;
        let destination = current_dir_path.join(dst.item.as_str());

        let path_last_char = destination.as_os_str().to_string_lossy().chars().last();
        let is_directory = path_last_char == Some('/') || path_last_char == Some('\\');
        if is_directory && !destination.exists() {
            return Err(ShellError::DirectoryNotFound {
                dir: destination.to_string_lossy().to_string(),
                span: dst.span,
            });
        }
        let ctrlc = engine_state.ctrlc.clone();
        let span = call.head;

        // Get an iterator with all the source files.
        let sources: Vec<_> = match arg_glob(&src, &current_dir_path) {
            Ok(files) => files.collect(),
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: e.to_string(),
                    msg: "invalid pattern".into(),
                    span: Some(src.span),
                    help: None,
                    inner: vec![],
                })
            }
        };

        if sources.is_empty() {
            return Err(ShellError::FileNotFound { span: src.span });
        }

        if sources.len() > 1 && !destination.is_dir() {
            return Err(ShellError::GenericError {
                error: "Destination must be a directory when copying multiple files".into(),
                msg: "is not a directory".into(),
                span: Some(dst.span),
                help: None,
                inner: vec![],
            });
        }

        let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));

        if any_source_is_dir && !recursive {
            return Err(ShellError::GenericError {
                error: "Directories must be copied using \"--recursive\"".into(),
                msg: "resolves to a directory (not copied)".into(),
                span: Some(src.span),
                help: None,
                inner: vec![],
            });
        }

        let mut result = Vec::new();

        for entry in sources.into_iter().flatten() {
            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                return Ok(PipelineData::empty());
            }

            let mut sources = FileStructure::new();
            sources.walk_decorate(&entry, engine_state, stack)?;

            if entry.is_file() {
                let sources = sources.paths_applying_with(|(source_file, _depth_level)| {
                    if destination.is_dir() {
                        let mut dest = canonicalize_with(&dst.item, &current_dir_path)?;
                        if let Some(name) = entry.file_name() {
                            dest.push(name);
                        }
                        Ok((source_file, dest))
                    } else {
                        Ok((source_file, destination.clone()))
                    }
                })?;

                for (src, dst) in sources {
                    if src.is_file() {
                        let dst =
                            canonicalize_with(dst.as_path(), &current_dir_path).unwrap_or(dst);

                        // ignore when source file is not newer than target file
                        if update_mode && super::util::is_older(&src, &dst).unwrap_or(false) {
                            continue;
                        }

                        let res = if src == dst {
                            let msg = format!("src and dst identical: {:?} (not copied)", src);

                            return Err(ShellError::GenericError {
                                error: "Copy aborted".into(),
                                msg,
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            });
                        } else if interactive && dst.exists() {
                            if progress {
                                interactive_copy(
                                    interactive,
                                    src,
                                    dst,
                                    span,
                                    &ctrlc,
                                    copy_file_with_progressbar,
                                )
                            } else {
                                interactive_copy(interactive, src, dst, span, &None, copy_file)
                            }
                        } else if progress {
                            // use std::io::copy to get the progress
                            // slower then std::fs::copy but useful if user needs to see the progress
                            copy_file_with_progressbar(src, dst, span, &ctrlc)
                        } else {
                            // use std::fs::copy
                            copy_file(src, dst, span, &None)
                        };
                        result.push(res);
                    }
                }
            } else if entry.is_dir() {
                let destination = if !destination.exists() {
                    destination.clone()
                } else {
                    match entry.file_name() {
                        Some(name) => destination.join(name),
                        None => {
                            return Err(ShellError::GenericError {
                                error: "Copy aborted. Not a valid path".into(),
                                msg: "not a valid path".into(),
                                span: Some(dst.span),
                                help: None,
                                inner: vec![],
                            })
                        }
                    }
                };

                std::fs::create_dir_all(&destination).map_err(|e| ShellError::GenericError {
                    error: e.to_string(),
                    msg: e.to_string(),
                    span: Some(dst.span),
                    help: None,
                    inner: vec![],
                })?;

                let not_follow_symlink = call.has_flag("no-symlink");
                let sources = sources.paths_applying_with(|(source_file, depth_level)| {
                    let mut dest = destination.clone();

                    let path = if not_follow_symlink {
                        expand_path_with(&source_file, &current_dir_path)
                    } else {
                        canonicalize_with(&source_file, &current_dir_path).or_else(|err| {
                            // check if dangling symbolic link.
                            let path = expand_path_with(&source_file, &current_dir_path);
                            if path.is_symlink() && !path.exists() {
                                Ok(path)
                            } else {
                                Err(err)
                            }
                        })?
                    };

                    #[allow(clippy::needless_collect)]
                    let comps: Vec<_> = path
                        .components()
                        .map(|fragment| fragment.as_os_str())
                        .rev()
                        .take(1 + depth_level)
                        .collect();

                    for fragment in comps.into_iter().rev() {
                        dest.push(fragment);
                    }

                    Ok((PathBuf::from(&source_file), dest))
                })?;

                for (s, d) in sources {
                    // Check if the user has pressed ctrl+c before copying a file
                    if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                        return Ok(PipelineData::empty());
                    }

                    if s.is_dir() && !d.exists() {
                        std::fs::create_dir_all(&d).map_err(|e| ShellError::GenericError {
                            error: e.to_string(),
                            msg: e.to_string(),
                            span: Some(dst.span),
                            help: None,
                            inner: vec![],
                        })?;
                    }
                    if s.is_symlink() && not_follow_symlink {
                        let res = if interactive && d.exists() {
                            interactive_copy(interactive, s, d, span, &None, copy_symlink)
                        } else {
                            copy_symlink(s, d, span, &None)
                        };
                        result.push(res);
                    } else if s.is_file() {
                        let res = if interactive && d.exists() {
                            if progress {
                                interactive_copy(
                                    interactive,
                                    s,
                                    d,
                                    span,
                                    &ctrlc,
                                    copy_file_with_progressbar,
                                )
                            } else {
                                interactive_copy(interactive, s, d, span, &None, copy_file)
                            }
                        } else if progress {
                            copy_file_with_progressbar(s, d, span, &ctrlc)
                        } else {
                            copy_file(s, d, span, &None)
                        };
                        result.push(res);
                    };
                }
            }
        }

        if verbose {
            result
                .into_iter()
                .into_pipeline_data(ctrlc)
                .print_not_formatted(engine_state, false, true)?;
        } else {
            // filter to only errors
            result
                .into_iter()
                .filter(|v| matches!(v, Value::Error { .. }))
                .into_pipeline_data(ctrlc)
                .print_not_formatted(engine_state, false, true)?;
        }
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Copy myfile to dir_b",
                example: "cp myfile dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b",
                example: "cp -r dir_a dir_b",
                result: None,
            },
            Example {
                description: "Recursively copy dir_a to dir_b, and print the feedbacks",
                example: "cp -r -v dir_a dir_b",
                result: None,
            },
            Example {
                description: "Move many files into a directory",
                example: "cp *.txt dir_a",
                result: None,
            },
            Example {
                description: "Copy only if source file is newer than target file",
                example: "cp -u a b",
                result: None,
            },
        ]
    }
}

fn interactive_copy(
    interactive: bool,
    src: PathBuf,
    dst: PathBuf,
    span: Span,
    _ctrl_status: &Option<Arc<AtomicBool>>,
    copy_impl: impl Fn(PathBuf, PathBuf, Span, &Option<Arc<AtomicBool>>) -> Value,
) -> Value {
    let (interaction, confirmed) = try_interaction(
        interactive,
        format!("cp: overwrite '{}'? ", dst.to_string_lossy()),
    );
    if let Err(e) = interaction {
        Value::error(
            ShellError::GenericError {
                error: e.to_string(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            span,
        )
    } else if !confirmed {
        let msg = format!("{:} not copied to {:}", src.display(), dst.display());
        Value::string(msg, span)
    } else {
        copy_impl(src, dst, span, &None)
    }
}

// This uses `std::fs::copy` to copy a file. There is another function called `copy_file_with_progressbar`
// which uses `read` and `write` instead. This is to get the progress of the copy. Try to keep the logic in
// this function in sync with `copy_file_with_progressbar`
// FIXME: `std::fs::copy` can't be interrupted. Consider using something else
fn copy_file(
    src: PathBuf,
    dst: PathBuf,
    span: Span,
    _ctrlc_status: &Option<Arc<AtomicBool>>,
) -> Value {
    match std::fs::copy(&src, &dst) {
        Ok(_) => {
            let msg = format!("copied {:} to {:}", src.display(), dst.display());
            Value::string(msg, span)
        }
        Err(e) => convert_io_error(e, src, dst, span),
    }
}

// This uses `read` and `write` to copy a file. There is another function called `copy_file`
// which uses `std::fs::copy` instead which is faster but does not provide progress updates for the copy. try to keep the
// logic in this function in sync with `copy_file`
fn copy_file_with_progressbar(
    src: PathBuf,
    dst: PathBuf,
    span: Span,
    ctrlc_status: &Option<Arc<AtomicBool>>,
) -> Value {
    let mut bytes_processed: u64 = 0;
    let mut process_failed: Option<std::io::Error> = None;

    let file_in = match std::fs::File::open(&src) {
        Ok(file) => file,
        Err(error) => return convert_io_error(error, src, dst, span),
    };

    let file_size = match file_in.metadata() {
        Ok(metadata) => Some(metadata.len()),
        _ => None,
    };

    let mut bar = progress_bar::NuProgressBar::new(file_size);

    let file_out = match std::fs::File::create(&dst) {
        Ok(file) => file,
        Err(error) => return convert_io_error(error, src, dst, span),
    };
    let mut buffer = [0u8; 8192];
    let mut buf_reader = BufReader::new(file_in);
    let mut buf_writer = BufWriter::new(file_out);

    loop {
        if nu_utils::ctrl_c::was_pressed(ctrlc_status) {
            let err = std::io::Error::new(ErrorKind::Interrupted, "Interrupted");
            process_failed = Some(err);
            break;
        }

        // Read src file
        match buf_reader.read(&mut buffer) {
            // src file read successfully
            Ok(bytes_read) => {
                // Write dst file
                match buf_writer.write(&buffer[..bytes_read]) {
                    // dst file written successfully
                    Ok(bytes_written) => {
                        // Update the total amount of bytes that has been saved and then print the progress bar
                        bytes_processed += bytes_written as u64;
                        bar.update_bar(bytes_processed);

                        // the last block of bytes is going to be lower than the buffer size
                        // let's break the loop once we write the last block
                        if bytes_read < buffer.len() {
                            break;
                        }
                    }
                    Err(e) => {
                        // There was a problem writing the dst file
                        process_failed = Some(e);
                        break;
                    }
                }
            }
            Err(e) => {
                // There was a problem reading the src file
                process_failed = Some(e);
                break;
            }
        };
    }

    // If copying the file failed
    if let Some(error) = process_failed {
        if error.kind() == ErrorKind::Interrupted {
            bar.abandoned_msg("# !! Interrupted !!".to_owned());
        } else {
            bar.abandoned_msg("# !! Error !!".to_owned());
        }
        return convert_io_error(error, src, dst, span);
    }

    // Get the name of the file to print it out at the end
    let file_name = &src
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new(""))
        .to_string_lossy();
    let msg = format!("copied {:} to {:}", src.display(), dst.display());
    bar.finished_msg(format!(" {} copied!", &file_name));

    Value::string(msg, span)
}

fn copy_symlink(
    src: PathBuf,
    dst: PathBuf,
    span: Span,
    _ctrlc_status: &Option<Arc<AtomicBool>>,
) -> Value {
    let target_path = read_link(src.as_path());
    let target_path = match target_path {
        Ok(p) => p,
        Err(e) => {
            return Value::error(
                ShellError::GenericError {
                    error: e.to_string(),
                    msg: e.to_string(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                },
                span,
            )
        }
    };

    let create_symlink = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink
        }

        #[cfg(windows)]
        {
            if !target_path.exists() || target_path.is_file() {
                std::os::windows::fs::symlink_file
            } else {
                std::os::windows::fs::symlink_dir
            }
        }
    };

    match create_symlink(target_path.as_path(), dst.as_path()) {
        Ok(_) => {
            let msg = format!("copied {:} to {:}", src.display(), dst.display());
            Value::string(msg, span)
        }
        Err(e) => Value::error(
            ShellError::GenericError {
                error: e.to_string(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            },
            span,
        ),
    }
}

// Function to convert io::Errors to more specific ShellErrors
fn convert_io_error(error: std::io::Error, src: PathBuf, dst: PathBuf, span: Span) -> Value {
    let message_src = format!(
        "copying file '{src_display}' failed: {error}",
        src_display = src.display()
    );

    let message_dst = format!(
        "copying to destination '{dst_display}' failed: {error}",
        dst_display = dst.display()
    );

    let shell_error = match error.kind() {
        ErrorKind::NotFound => {
            if std::path::Path::new(&dst).exists() {
                ShellError::FileNotFoundCustom {
                    msg: message_src,
                    span,
                }
            } else {
                ShellError::FileNotFoundCustom {
                    msg: message_dst,
                    span,
                }
            }
        }
        ErrorKind::PermissionDenied => match std::fs::metadata(&dst) {
            Ok(meta) => {
                if meta.permissions().readonly() {
                    ShellError::PermissionDeniedError {
                        msg: message_dst,
                        span,
                    }
                } else {
                    ShellError::PermissionDeniedError {
                        msg: message_src,
                        span,
                    }
                }
            }
            Err(_) => ShellError::PermissionDeniedError {
                msg: message_dst,
                span,
            },
        },
        ErrorKind::Interrupted => ShellError::IOInterrupted {
            msg: message_src,
            span,
        },
        ErrorKind::OutOfMemory => ShellError::OutOfMemoryError {
            msg: message_src,
            span,
        },
        // TODO: handle ExecutableFileBusy etc. when io_error_more is stabilized
        // https://github.com/rust-lang/rust/issues/86442
        _ => ShellError::IOErrorSpanned {
            msg: message_src,
            span,
        },
    };

    Value::error(shell_error, span)
}
