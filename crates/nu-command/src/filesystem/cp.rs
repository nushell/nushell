use std::cell::RefCell;
use std::fs::read_link;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
use indicatif::{MultiProgress, ProgressBar};

const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
    recursive_match_hidden_dir: true,
};

#[derive(Clone)]
pub struct Cp;

impl Command for Cp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["copy", "file", "files"]
    }

    fn signature(&self) -> Signature {
        Signature::build("cp")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required("source", SyntaxShape::GlobPattern, "the place to copy from")
            .required("destination", SyntaxShape::Filepath, "the place to copy to")
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
        let src = {
            Spanned {
                item: nu_utils::strip_ansi_string_unlikely(src.item),
                span: src.span,
            }
        };
        let dst: Spanned<String> = call.req(engine_state, stack, 1)?;
        let recursive = call.has_flag("recursive");
        let verbose = call.has_flag("verbose");
        let interactive = call.has_flag("interactive");
        let progress = call.has_flag("progress");

        let current_dir_path = current_dir(engine_state, stack)?;
        let source = current_dir_path.join(src.item.as_str());
        let destination = current_dir_path.join(dst.item.as_str());

        let path_last_char = destination.as_os_str().to_string_lossy().chars().last();
        let is_directory = path_last_char == Some('/') || path_last_char == Some('\\');
        if is_directory && !destination.exists() {
            return Err(ShellError::DirectoryNotFound(
                dst.span,
                Some("destination directory does not exist".to_string()),
            ));
        }
        let ctrlc = engine_state.ctrlc.clone();
        let span = call.head;

        // Get an iterator with all the source files.
        let sources: Vec<_> = match nu_glob::glob_with(&source.to_string_lossy(), GLOB_PARAMS) {
            Ok(files) => files.collect(),
            Err(e) => {
                return Err(ShellError::GenericError(
                    e.to_string(),
                    "invalid pattern".to_string(),
                    Some(src.span),
                    None,
                    Vec::new(),
                ))
            }
        };

        if sources.is_empty() {
            return Err(ShellError::GenericError(
                "No matches found".into(),
                "no matches found".into(),
                Some(src.span),
                None,
                Vec::new(),
            ));
        }

        if sources.len() > 1 && !destination.is_dir() {
            return Err(ShellError::GenericError(
                "Destination must be a directory when copying multiple files".into(),
                "is not a directory".into(),
                Some(dst.span),
                None,
                Vec::new(),
            ));
        }

        let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));

        if any_source_is_dir && !recursive {
            return Err(ShellError::GenericError(
                "Directories must be copied using \"--recursive\"".into(),
                "resolves to a directory (not copied)".into(),
                Some(src.span),
                None,
                Vec::new(),
            ));
        }

        let mut result = Vec::new();
        // Create the progress bar holder and the styles
        let (_multi_pb, pb_overall, pb_perfile, style_overall) = if progress {
            let tmp_multi_pb = MultiProgress::new();

            // Progress bar to show the status for all files
            let tmp_pb_overall = tmp_multi_pb.add(ProgressBar::new(0));
            tmp_pb_overall.set_style(progress_bar::nu_progress_style(
                progress_bar::ProgressType::Unknown,
            ));

            // Progress for each file
            let tmp_pb_perfile = tmp_multi_pb.insert_after(&tmp_pb_overall, ProgressBar::new(0));

            (
                Some(tmp_multi_pb),
                Some(tmp_pb_overall),
                Some(tmp_pb_perfile),
                Some(progress_bar::nu_progress_style(
                    progress_bar::ProgressType::Items,
                )),
            )
        } else {
            (None, None, None, None)
        };

        let mut src_n_files = sources.len() as u64;

        if let (Some(pb_overall), Some(style_overall)) = (&pb_overall, style_overall.clone()) {
            if src_n_files == 1 {
                src_n_files = 0;
            }

            pb_overall.set_style(style_overall);
            pb_overall.set_message("Copying files...");
            pb_overall.set_length(src_n_files);
        }

        for entry in sources.into_iter().flatten() {
            if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                if let Some(pb_overall) = &pb_overall {
                    pb_overall.abandon_with_message("Copy cancelled");
                }
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
                    // Update the overall progress bar
                    if let Some(pb_overall) = &pb_overall {
                        pb_overall.inc(1);
                    }

                    if src.is_file() {
                        let dst =
                            canonicalize_with(dst.as_path(), &current_dir_path).unwrap_or(dst);
                        let res = if src == dst {
                            let message = format!(
                                "src {source:?} and dst {destination:?} are identical(not copied)"
                            );

                            return Err(ShellError::GenericError(
                                "Copy aborted".into(),
                                message,
                                Some(span),
                                None,
                                Vec::new(),
                            ));
                        } else if interactive && dst.exists() {
                            // If progress bar is set
                            if let Some(pb_perfile) = &pb_perfile {
                                interactive_copy(
                                    interactive,
                                    src,
                                    dst,
                                    span,
                                    &ctrlc,
                                    Some(pb_perfile),
                                    copy_file_with_progressbar,
                                )
                            } else {
                                interactive_copy(
                                    interactive,
                                    src,
                                    dst,
                                    span,
                                    &None,
                                    None,
                                    copy_file,
                                )
                            }
                        } else if let Some(pb_perfile) = &pb_perfile {
                            // use std::io::copy to get the progress
                            // slower then std::fs::copy but useful if user needs to see the progress
                            copy_file_with_progressbar(src, dst, span, &ctrlc, Some(pb_perfile))
                        } else {
                            // use std::fs::copy
                            copy_file(src, dst, span, &None, None)
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
                            return Err(ShellError::GenericError(
                                "Copy aborted. Not a valid path".into(),
                                "not a valid path".into(),
                                Some(dst.span),
                                None,
                                Vec::new(),
                            ))
                        }
                    }
                };

                std::fs::create_dir_all(&destination).map_err(|e| {
                    ShellError::GenericError(
                        e.to_string(),
                        e.to_string(),
                        Some(dst.span),
                        None,
                        Vec::new(),
                    )
                })?;

                // TODO: if progress
                // variables used to cancel copy progress.
                let is_copy_cancelled = Rc::new(RefCell::new(false));
                let is_copy_cancelled_clone = is_copy_cancelled.clone();

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

                    if let Some(pb_overall) = &pb_overall {
                        pb_overall.set_message("Gathering file path list...".to_string());
                    }

                    let mut is_copy_cancelled_clone = is_copy_cancelled_clone.borrow_mut();
                    for fragment in comps.into_iter().rev() {
                        if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                            *is_copy_cancelled_clone = true;

                            // TBD - we might want to be able to cancel the copy with this method
                            // but if we do this we are going to have inconsistencies on the UI
                            // return Err(Box::new(ShellError::IOInterrupted(
                            //     "copy cancelled".to_string(),
                            //     span,
                            // )));
                        }

                        if let Some(pb_overall) = &pb_overall {
                            pb_overall.tick();
                        }
                        dest.push(fragment);
                    }

                    Ok((PathBuf::from(&source_file), dest))
                })?;

                // If this is true, ctrl+c was pressed while the source list
                // was being built. Cancel the copy.
                if *is_copy_cancelled.borrow() {
                    if let Some(pb_overall) = &pb_overall {
                        pb_overall.abandon_with_message("Copy cancelled");
                    }
                    return Ok(PipelineData::empty());
                }

                if let (Some(pb_overall), Some(style_overall)) =
                    (&pb_overall, style_overall.clone())
                {
                    src_n_files += sources.len() as u64;
                    pb_overall.set_style(style_overall);
                    pb_overall.set_length(src_n_files);
                    pb_overall.set_message("Copying files...");
                }

                for (s, d) in sources {
                    // Update the overall progress bar
                    if let Some(pb_overall) = &pb_overall {
                        pb_overall.inc(1);
                    }

                    // Check if the user has pressed ctrl+c before copying a file
                    if nu_utils::ctrl_c::was_pressed(&ctrlc) {
                        if let Some(pb_overall) = pb_overall {
                            pb_overall.abandon_with_message("Copy interrupted");
                        }
                        return Ok(PipelineData::empty());
                    }

                    if s.is_dir() && !d.exists() {
                        std::fs::create_dir_all(&d).map_err(|e| {
                            ShellError::GenericError(
                                e.to_string(),
                                e.to_string(),
                                Some(dst.span),
                                None,
                                Vec::new(),
                            )
                        })?;
                    }
                    if s.is_symlink() && not_follow_symlink {
                        let res = if interactive && d.exists() {
                            interactive_copy(interactive, s, d, span, &None, None, copy_symlink)
                        } else {
                            copy_symlink(s, d, span, &None, None)
                        };
                        result.push(res);
                    } else if s.is_file() {
                        let res = if interactive && d.exists() {
                            if let Some(pb_perfile) = &pb_perfile {
                                interactive_copy(
                                    interactive,
                                    s,
                                    d,
                                    span,
                                    &ctrlc,
                                    Some(pb_perfile),
                                    copy_file_with_progressbar,
                                )
                            } else {
                                interactive_copy(interactive, s, d, span, &None, None, copy_file)
                            }
                        } else if let Some(pb_perfile) = &pb_perfile {
                            copy_file_with_progressbar(s, d, span, &ctrlc, Some(pb_perfile))
                        } else {
                            copy_file(s, d, span, &None, None)
                        };
                        result.push(res);
                    };
                }
            }
        }

        if let (Some(pb_overall), Some(pb_perfile)) = (pb_overall, pb_perfile) {
            pb_perfile.finish_and_clear();
            pb_overall.finish_with_message("Files successfully copied!");
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
        ]
    }
}

fn interactive_copy(
    interactive: bool,
    src: PathBuf,
    dst: PathBuf,
    span: Span,
    _ctrl_status: &Option<Arc<AtomicBool>>,
    pb: Option<&ProgressBar>,
    copy_impl: impl Fn(PathBuf, PathBuf, Span, &Option<Arc<AtomicBool>>, Option<&ProgressBar>) -> Value,
) -> Value {
    let (interaction, confirmed) = try_interaction(
        interactive,
        format!("cp: overwrite '{}'? ", dst.to_string_lossy()),
    );
    if let Err(e) = interaction {
        Value::Error {
            error: ShellError::GenericError(
                e.to_string(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            ),
        }
    } else if !confirmed {
        let msg = format!("{:} not copied to {:}", src.display(), dst.display());
        Value::String { val: msg, span }
    } else {
        copy_impl(src, dst, span, &None, pb)
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
    _pb: Option<&ProgressBar>,
) -> Value {
    match std::fs::copy(&src, &dst) {
        Ok(_) => {
            let msg = format!("copied {:} to {:}", src.display(), dst.display());
            Value::String { val: msg, span }
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
    pb: Option<&ProgressBar>,
) -> Value {
    let pb = pb.expect("Error: Missing progress bar");
    let mut bytes_processed: u64 = 0;
    let mut process_failed: Option<std::io::Error> = None;
    let file_name = &src
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new(""))
        .to_string_lossy();

    let file_in = match std::fs::File::open(&src) {
        Ok(file) => file,
        Err(error) => return convert_io_error(error, src, dst, span),
    };

    match file_in.metadata() {
        Ok(metadata) => {
            pb.set_style(progress_bar::nu_progress_style(
                progress_bar::ProgressType::Bytes,
            ));
            pb.set_length(metadata.len());
            //Some(metadata.len())
        }
        _ => {
            pb.set_style(progress_bar::nu_progress_style(
                progress_bar::ProgressType::BytesUnknown,
            ));
        }
    };

    pb.set_message(file_name.to_string());

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
                        pb.set_position(bytes_processed);

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
            pb.abandon_with_message("# !! Interrupted !!".to_owned());
        } else {
            pb.abandon_with_message("# !! Error !!".to_owned());
        }
        return convert_io_error(error, src, dst, span);
    }

    let msg = format!("copied {:} to {:}", src.display(), dst.display());
    pb.finish_with_message(format!(" {} copied!", &file_name));
    pb.reset_elapsed();

    Value::String { val: msg, span }
}

fn copy_symlink(
    src: PathBuf,
    dst: PathBuf,
    span: Span,
    _ctrlc_status: &Option<Arc<AtomicBool>>,
    _pb: Option<&ProgressBar>,
) -> Value {
    let target_path = read_link(src.as_path());
    let target_path = match target_path {
        Ok(p) => p,
        Err(err) => {
            return Value::Error {
                error: ShellError::GenericError(
                    err.to_string(),
                    err.to_string(),
                    Some(span),
                    None,
                    vec![],
                ),
            }
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
            Value::String { val: msg, span }
        }
        Err(e) => Value::Error {
            error: ShellError::GenericError(e.to_string(), e.to_string(), Some(span), None, vec![]),
        },
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
                ShellError::FileNotFoundCustom(message_src, span)
            } else {
                ShellError::FileNotFoundCustom(message_dst, span)
            }
        }
        ErrorKind::PermissionDenied => match std::fs::metadata(&dst) {
            Ok(meta) => {
                if meta.permissions().readonly() {
                    ShellError::PermissionDeniedError(message_dst, span)
                } else {
                    ShellError::PermissionDeniedError(message_src, span)
                }
            }
            Err(_) => ShellError::PermissionDeniedError(message_dst, span),
        },
        ErrorKind::Interrupted => ShellError::IOInterrupted(message_src, span),
        ErrorKind::OutOfMemory => ShellError::OutOfMemoryError(message_src, span),
        // TODO: handle ExecutableFileBusy etc. when io_error_more is stabilized
        // https://github.com/rust-lang/rust/issues/86442
        _ => ShellError::IOErrorSpanned(message_src, span),
    };

    Value::Error { error: shell_error }
}
