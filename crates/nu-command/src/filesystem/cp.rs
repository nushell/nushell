use std::fs::read_link;
use std::path::PathBuf;

use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_path::{canonicalize_with, expand_path_with};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value,
};

use super::util::try_interaction;

use crate::filesystem::util::FileStructure;

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

        for entry in sources.into_iter().flatten() {
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
                        let res = if src == dst {
                            let message = format!(
                                "src {:?} and dst {:?} are identical(not copied)",
                                source, destination
                            );

                            return Err(ShellError::GenericError(
                                "Copy aborted".into(),
                                message,
                                Some(span),
                                None,
                                Vec::new(),
                            ));
                        } else if interactive && dst.exists() {
                            interactive_copy(interactive, src, dst, span, copy_file)
                        } else {
                            copy_file(src, dst, span)
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
                            interactive_copy(interactive, s, d, span, copy_symlink)
                        } else {
                            copy_symlink(s, d, span)
                        };
                        result.push(res);
                    } else if s.is_file() {
                        let res = if interactive && d.exists() {
                            interactive_copy(interactive, s, d, span, copy_file)
                        } else {
                            copy_file(s, d, span)
                        };
                        result.push(res);
                    };
                }
            }
        }

        if verbose {
            Ok(result.into_iter().into_pipeline_data(ctrlc))
        } else {
            // filter to only errors
            Ok(result
                .into_iter()
                .filter(|v| matches!(v, Value::Error { .. }))
                .into_pipeline_data(ctrlc))
        }
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
    copy_impl: impl Fn(PathBuf, PathBuf, Span) -> Value,
) -> Value {
    let (interaction, confirmed) =
        try_interaction(interactive, "cp: overwrite", &dst.to_string_lossy());
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
        copy_impl(src, dst, span)
    }
}

fn copy_file(src: PathBuf, dst: PathBuf, span: Span) -> Value {
    match std::fs::copy(&src, &dst) {
        Ok(_) => {
            let msg = format!("copied {:} to {:}", src.display(), dst.display());
            Value::String { val: msg, span }
        }
        Err(e) => {
            let message = format!("copy file {src:?} failed: {e}");

            use std::io::ErrorKind;
            let shell_error = match e.kind() {
                ErrorKind::NotFound => ShellError::FileNotFoundCustom(message, span),
                ErrorKind::PermissionDenied => ShellError::PermissionDeniedError(message, span),
                ErrorKind::Interrupted => ShellError::IOInterrupted(message, span),
                ErrorKind::OutOfMemory => ShellError::OutOfMemoryError(message, span),
                // TODO: handle ExecutableFileBusy etc. when io_error_more is stabilized
                // https://github.com/rust-lang/rust/issues/86442
                _ => ShellError::IOErrorSpanned(message, span),
            };

            Value::Error { error: shell_error }
        }
    }
}

fn copy_symlink(src: PathBuf, dst: PathBuf, span: Span) -> Value {
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
