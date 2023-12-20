use std::collections::HashMap;
use std::io::Error;
use std::io::ErrorKind;
#[cfg(unix)]
use std::os::unix::prelude::FileTypeExt;
use std::path::PathBuf;

use super::util::try_interaction;

use nu_cmd_base::arg_glob_leading_dot;
use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};

const TRASH_SUPPORTED: bool = cfg!(all(
    feature = "trash-support",
    not(any(target_os = "android", target_os = "ios"))
));

#[derive(Clone)]
pub struct Rm;

impl Command for Rm {
    fn name(&self) -> &str {
        "rm"
    }

    fn usage(&self) -> &str {
        "Remove files and directories."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete", "remove"]
    }

    fn signature(&self) -> Signature {
        let sig = Signature::build("rm")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .required(
                "filename",
                SyntaxShape::GlobPattern,
                "The file or files you want to remove.",
            )
            .switch(
                "trash",
                "move to the platform's trash instead of permanently deleting. not used on android and ios",
                Some('t'),
            )
            .switch(
                "permanent",
                "delete permanently, ignoring the 'always_trash' config option. always enabled on android and ios",
                Some('p'),
            );
        sig.switch("recursive", "delete subdirectories recursively", Some('r'))
            .switch("force", "suppress error when no file", Some('f'))
            .switch("verbose", "print names of deleted files", Some('v'))
            .switch("interactive", "ask user to confirm action", Some('i'))
            .switch(
                "interactive-once",
                "ask user to confirm action only once",
                Some('I'),
            )
            .rest(
                "rest",
                SyntaxShape::GlobPattern,
                "Additional file path(s) to remove.",
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
        rm(engine_state, stack, call)
    }

    fn examples(&self) -> Vec<Example> {
        let mut examples = vec![Example {
            description:
                "Delete, or move a file to the trash (based on the 'always_trash' config option)",
            example: "rm file.txt",
            result: None,
        }];
        if TRASH_SUPPORTED {
            examples.append(&mut vec![
                Example {
                    description: "Move a file to the trash",
                    example: "rm --trash file.txt",
                    result: None,
                },
                Example {
                    description:
                        "Delete a file permanently, even if the 'always_trash' config option is true",
                    example: "rm --permanent file.txt",
                    result: None,
                },
            ]);
        }
        examples.push(Example {
            description: "Delete a file, ignoring 'file not found' errors",
            example: "rm --force file.txt",
            result: None,
        });
        examples.push(Example {
            description: "Delete all 0KB files in the current directory",
            example: "ls | where size == 0KB and type == file | each { rm $in.name } | null",
            result: None,
        });
        examples
    }
}

fn rm(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let trash = call.has_flag("trash");
    let permanent = call.has_flag("permanent");
    let recursive = call.has_flag("recursive");
    let force = call.has_flag("force");
    let verbose = call.has_flag("verbose");
    let interactive = call.has_flag("interactive");
    let interactive_once = call.has_flag("interactive-once") && !interactive;

    let ctrlc = engine_state.ctrlc.clone();

    let mut targets: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

    let mut unique_argument_check = None;

    let currentdir_path = current_dir(engine_state, stack)?;

    let home: Option<String> = nu_path::home_dir().map(|path| {
        {
            if path.exists() {
                match nu_path::canonicalize_with(&path, &currentdir_path) {
                    Ok(canon_path) => canon_path,
                    Err(_) => path,
                }
            } else {
                path
            }
        }
        .to_string_lossy()
        .into()
    });

    for (idx, path) in targets.clone().into_iter().enumerate() {
        if let Some(ref home) = home {
            if &path.item == home {
                unique_argument_check = Some(path.span);
            }
        }
        let corrected_path = Spanned {
            item: nu_utils::strip_ansi_string_unlikely(path.item),
            span: path.span,
        };
        let _ = std::mem::replace(&mut targets[idx], corrected_path);
    }

    let span = call.head;
    let rm_always_trash = engine_state.get_config().rm_always_trash;

    if !TRASH_SUPPORTED {
        if rm_always_trash {
            return Err(ShellError::GenericError {
                error: "Cannot execute `rm`; the current configuration specifies \
                    `always_trash = true`, but the current nu executable was not \
                    built with feature `trash_support`."
                    .into(),
                msg: "trash required to be true but not supported".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            });
        } else if trash {
            return Err(ShellError::GenericError{
                error: "Cannot execute `rm` with option `--trash`; feature `trash-support` not enabled or on an unsupported platform"
                    .into(),
                msg: "this option is only available if nu is built with the `trash-support` feature and the platform supports trash"
                    .into(),
                span: Some(span),
                help: None,
                inner: vec![],
            });
        }
    }

    if targets.is_empty() {
        return Err(ShellError::GenericError {
            error: "rm requires target paths".into(),
            msg: "needs parameter".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        });
    }

    if unique_argument_check.is_some() && !(interactive_once || interactive) {
        return Err(ShellError::GenericError {
            error: "You are trying to remove your home dir".into(),
            msg: "If you really want to remove your home dir, please use -I or -i".into(),
            span: unique_argument_check,
            help: None,
            inner: vec![],
        });
    }

    let targets_span = Span::new(
        targets
            .iter()
            .map(|x| x.span.start)
            .min()
            .expect("targets were empty"),
        targets
            .iter()
            .map(|x| x.span.end)
            .max()
            .expect("targets were empty"),
    );

    let (mut target_exists, mut empty_span) = (false, call.head);
    let mut all_targets: HashMap<PathBuf, Span> = HashMap::new();

    for target in targets {
        if currentdir_path.to_string_lossy() == target.item
            || currentdir_path.starts_with(format!("{}{}", target.item, std::path::MAIN_SEPARATOR))
        {
            return Err(ShellError::GenericError {
                error: "Cannot remove any parent directory".into(),
                msg: "cannot remove any parent directory".into(),
                span: Some(target.span),
                help: None,
                inner: vec![],
            });
        }

        let path = currentdir_path.join(&target.item);
        match arg_glob_leading_dot(&target, &currentdir_path) {
            Ok(files) => {
                for file in files {
                    match file {
                        Ok(f) => {
                            if !target_exists {
                                target_exists = true;
                            }

                            // It is not appropriate to try and remove the
                            // current directory or its parent when using
                            // glob patterns.
                            let name = f.display().to_string();
                            if name.ends_with("/.") || name.ends_with("/..") {
                                continue;
                            }

                            all_targets
                                .entry(nu_path::expand_path_with(f, &currentdir_path))
                                .or_insert_with(|| target.span);
                        }
                        Err(e) => {
                            return Err(ShellError::GenericError {
                                error: format!("Could not remove {:}", path.to_string_lossy()),
                                msg: e.to_string(),
                                span: Some(target.span),
                                help: None,
                                inner: vec![],
                            });
                        }
                    }
                }

                // Target doesn't exists
                if !target_exists && empty_span.eq(&call.head) {
                    empty_span = target.span;
                }
            }
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: e.to_string(),
                    msg: e.to_string(),
                    span: Some(target.span),
                    help: None,
                    inner: vec![],
                })
            }
        };
    }

    if all_targets.is_empty() && !force {
        return Err(ShellError::GenericError {
            error: "File(s) not found".into(),
            msg: "File(s) not found".into(),
            span: Some(targets_span),
            help: None,
            inner: vec![],
        });
    }

    if interactive_once {
        let (interaction, confirmed) = try_interaction(
            interactive_once,
            format!("rm: remove {} files? ", all_targets.len()),
        );
        if let Err(e) = interaction {
            return Err(ShellError::GenericError {
                error: format!("Error during interaction: {e:}"),
                msg: "could not move".into(),
                span: None,
                help: None,
                inner: vec![],
            });
        } else if !confirmed {
            return Ok(PipelineData::Empty);
        }
    }

    all_targets
        .into_iter()
        .map(move |(f, span)| {
            let is_empty = || match f.read_dir() {
                Ok(mut p) => p.next().is_none(),
                Err(_) => false,
            };

            if let Ok(metadata) = f.symlink_metadata() {
                #[cfg(unix)]
                let is_socket = metadata.file_type().is_socket();
                #[cfg(unix)]
                let is_fifo = metadata.file_type().is_fifo();

                #[cfg(not(unix))]
                let is_socket = false;
                #[cfg(not(unix))]
                let is_fifo = false;

                if metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || recursive
                    || is_socket
                    || is_fifo
                    || is_empty()
                {
                    let (interaction, confirmed) = try_interaction(
                        interactive,
                        format!("rm: remove '{}'? ", f.to_string_lossy()),
                    );

                    let result = if let Err(e) = interaction {
                        let e = Error::new(ErrorKind::Other, &*e.to_string());
                        Err(e)
                    } else if interactive && !confirmed {
                        Ok(())
                    } else if TRASH_SUPPORTED && (trash || (rm_always_trash && !permanent)) {
                        #[cfg(all(
                            feature = "trash-support",
                            not(any(target_os = "android", target_os = "ios"))
                        ))]
                        {
                            trash::delete(&f).map_err(|e: trash::Error| {
                                Error::new(ErrorKind::Other, format!("{e:?}\nTry '--trash' flag"))
                            })
                        }

                        // Should not be reachable since we error earlier if
                        // these options are given on an unsupported platform
                        #[cfg(any(
                            not(feature = "trash-support"),
                            target_os = "android",
                            target_os = "ios"
                        ))]
                        {
                            unreachable!()
                        }
                    } else if metadata.is_file()
                        || is_socket
                        || is_fifo
                        || metadata.file_type().is_symlink()
                    {
                        std::fs::remove_file(&f)
                    } else {
                        std::fs::remove_dir_all(&f)
                    };

                    if let Err(e) = result {
                        let msg = format!("Could not delete {:}: {e:}", f.to_string_lossy());
                        Value::error(ShellError::RemoveNotPossible { msg, span }, span)
                    } else if verbose {
                        let msg = if interactive && !confirmed {
                            "not deleted"
                        } else {
                            "deleted"
                        };
                        let val = format!("{} {:}", msg, f.to_string_lossy());
                        Value::string(val, span)
                    } else {
                        Value::nothing(span)
                    }
                } else {
                    let error = format!("Cannot remove {:}. try --recursive", f.to_string_lossy());
                    Value::error(
                        ShellError::GenericError {
                            error,
                            msg: "cannot remove non-empty directory".into(),
                            span: Some(span),
                            help: None,
                            inner: vec![],
                        },
                        span,
                    )
                }
            } else {
                let error = format!("no such file or directory: {:}", f.to_string_lossy());
                Value::error(
                    ShellError::GenericError {
                        error,
                        msg: "no such file or directory".into(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    },
                    span,
                )
            }
        })
        .filter(|x| !matches!(x.get_type(), Type::Nothing))
        .into_pipeline_data(ctrlc)
        .print_not_formatted(engine_state, false, true)?;

    Ok(PipelineData::empty())
}
