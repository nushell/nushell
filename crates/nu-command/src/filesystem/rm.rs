use super::util::try_interaction;
#[allow(deprecated)]
use nu_engine::{command_prelude::*, env::current_dir};
use nu_glob::MatchOptions;
use nu_path::expand_path_with;
use nu_protocol::{
    NuGlob, report_shell_error,
    shell_error::{self, io::IoError},
};
#[cfg(unix)]
use std::os::unix::prelude::FileTypeExt;
use std::{collections::HashMap, io::Error, path::PathBuf};

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

    fn description(&self) -> &str {
        "Remove files and directories."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["delete", "remove"]
    }

    fn signature(&self) -> Signature {
        Signature::build("rm")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest("paths", SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::String]), "The file paths(s) to remove.")
            .switch(
                "trash",
                "move to the platform's trash instead of permanently deleting. not used on android and ios",
                Some('t'),
            )
            .switch(
                "permanent",
                "delete permanently, ignoring the 'always_trash' config option. always enabled on android and ios",
                Some('p'),
            )
            .switch("recursive", "delete subdirectories recursively", Some('r'))
            .switch("force", "suppress error when no file", Some('f'))
            .switch("verbose", "print names of deleted files", Some('v'))
            .switch("interactive", "ask user to confirm action", Some('i'))
            .switch(
                "interactive-once",
                "ask user to confirm action only once",
                Some('I'),
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

    fn examples(&self) -> Vec<Example<'_>> {
        let mut examples = vec![Example {
            description: "Delete, or move a file to the trash (based on the 'always_trash' config option)",
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
    let trash = call.has_flag(engine_state, stack, "trash")?;
    let permanent = call.has_flag(engine_state, stack, "permanent")?;
    let recursive = call.has_flag(engine_state, stack, "recursive")?;
    let force = call.has_flag(engine_state, stack, "force")?;
    let verbose = call.has_flag(engine_state, stack, "verbose")?;
    let interactive = call.has_flag(engine_state, stack, "interactive")?;
    let interactive_once = call.has_flag(engine_state, stack, "interactive-once")? && !interactive;

    let mut paths = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;

    if paths.is_empty() {
        return Err(ShellError::MissingParameter {
            param_name: "requires file paths".to_string(),
            span: call.head,
        });
    }

    let mut unique_argument_check = None;

    #[allow(deprecated)]
    let currentdir_path = current_dir(engine_state, stack)?;

    let home: Option<String> = nu_path::home_dir().map(|path| {
        {
            if path.exists() {
                nu_path::canonicalize_with(&path, &currentdir_path).unwrap_or(path.into())
            } else {
                path.into()
            }
        }
        .to_string_lossy()
        .into()
    });

    for (idx, path) in paths.clone().into_iter().enumerate() {
        if let Some(ref home) = home
            && expand_path_with(path.item.as_ref(), &currentdir_path, path.item.is_expand())
                .to_string_lossy()
                .as_ref()
                == home.as_str()
        {
            unique_argument_check = Some(path.span);
        }
        let corrected_path = Spanned {
            item: match path.item {
                NuGlob::DoNotExpand(s) => {
                    NuGlob::DoNotExpand(nu_utils::strip_ansi_string_unlikely(s))
                }
                NuGlob::Expand(s) => NuGlob::Expand(nu_utils::strip_ansi_string_unlikely(s)),
            },
            span: path.span,
        };
        let _ = std::mem::replace(&mut paths[idx], corrected_path);
    }

    let span = call.head;
    let rm_always_trash = stack.get_config(engine_state).rm.always_trash;

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

    if paths.is_empty() {
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
        paths
            .iter()
            .map(|x| x.span.start)
            .min()
            .expect("targets were empty"),
        paths
            .iter()
            .map(|x| x.span.end)
            .max()
            .expect("targets were empty"),
    );

    let (mut target_exists, mut empty_span) = (false, call.head);
    let mut all_targets: HashMap<PathBuf, Span> = HashMap::new();

    for target in paths {
        let path = expand_path_with(
            target.item.as_ref(),
            &currentdir_path,
            target.item.is_expand(),
        );
        if currentdir_path.to_string_lossy() == path.to_string_lossy()
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

        match nu_engine::glob_from(
            &target,
            &currentdir_path,
            call.head,
            Some(MatchOptions {
                require_literal_leading_dot: true,
                ..Default::default()
            }),
            engine_state.signals().clone(),
        ) {
            Ok(files) => {
                for file in files.1 {
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
                                .entry(nu_path::expand_path_with(
                                    f,
                                    &currentdir_path,
                                    target.item.is_expand(),
                                ))
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
                // glob_from may canonicalize path and return an error when a directory is not found
                // nushell should suppress the error if `--force` is used.
                if !(force
                    && matches!(
                        e,
                        ShellError::Io(IoError {
                            kind: shell_error::io::ErrorKind::Std(std::io::ErrorKind::NotFound, ..),
                            ..
                        })
                    ))
                {
                    return Err(e);
                }
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
            return Ok(PipelineData::empty());
        }
    }

    let iter = all_targets.into_iter().map(move |(f, span)| {
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
                    Err(Error::other(&*e.to_string()))
                } else if interactive && !confirmed {
                    Ok(())
                } else if TRASH_SUPPORTED && (trash || (rm_always_trash && !permanent)) {
                    #[cfg(all(
                        feature = "trash-support",
                        not(any(target_os = "android", target_os = "ios"))
                    ))]
                    {
                        trash::delete(&f).map_err(|e: trash::Error| {
                            Error::other(format!("{e:?}\nTry '--permanent' flag"))
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
                } else if metadata.is_symlink() {
                    // In Windows, symlink pointing to a directory can be removed using
                    // std::fs::remove_dir instead of std::fs::remove_file.
                    #[cfg(windows)]
                    {
                        f.metadata().and_then(|metadata| {
                            if metadata.is_dir() {
                                std::fs::remove_dir(&f)
                            } else {
                                std::fs::remove_file(&f)
                            }
                        })
                    }

                    #[cfg(not(windows))]
                    std::fs::remove_file(&f)
                } else if metadata.is_file() || is_socket || is_fifo {
                    std::fs::remove_file(&f)
                } else {
                    std::fs::remove_dir_all(&f)
                };

                if let Err(e) = result {
                    Err(ShellError::Io(IoError::new(e, span, f)))
                } else if verbose {
                    let msg = if interactive && !confirmed {
                        "not deleted"
                    } else {
                        "deleted"
                    };
                    Ok(Some(format!("{} {:}", msg, f.to_string_lossy())))
                } else {
                    Ok(None)
                }
            } else {
                let error = format!("Cannot remove {:}. try --recursive", f.to_string_lossy());
                Err(ShellError::GenericError {
                    error,
                    msg: "cannot remove non-empty directory".into(),
                    span: Some(span),
                    help: None,
                    inner: vec![],
                })
            }
        } else {
            let error = format!("no such file or directory: {:}", f.to_string_lossy());
            Err(ShellError::GenericError {
                error,
                msg: "no such file or directory".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            })
        }
    });

    for result in iter {
        engine_state.signals().check(&call.head)?;
        match result {
            Ok(None) => {}
            Ok(Some(msg)) => eprintln!("{msg}"),
            Err(err) => report_shell_error(engine_state, &err),
        }
    }

    Ok(PipelineData::empty())
}
