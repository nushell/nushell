use std::path::PathBuf;

use nu_engine::env::current_dir;
use nu_engine::CallExt;
use nu_path::canonicalize_with;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

use crate::filesystem::util::FileStructure;

const GLOB_PARAMS: nu_glob::MatchOptions = nu_glob::MatchOptions {
    case_sensitive: true,
    require_literal_separator: false,
    require_literal_leading_dot: false,
};

#[derive(Clone)]
pub struct Cp;

#[allow(unused_must_use)]
impl Command for Cp {
    fn name(&self) -> &str {
        "cp"
    }

    fn usage(&self) -> &str {
        "Copy files."
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
            // TODO: add back in additional features
            // .switch("force", "suppress error when no file", Some('f'))
            // .switch("interactive", "ask user to confirm action", Some('i'))
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

        let path = current_dir(engine_state, stack)?;
        let source = path.join(src.item.as_str());
        let destination = path.join(dst.item.as_str());

        let sources: Vec<_> = match nu_glob::glob_with(&source.to_string_lossy(), GLOB_PARAMS) {
            Ok(files) => files.collect(),
            Err(e) => {
                return Err(ShellError::SpannedLabeledError(
                    e.to_string(),
                    "invalid pattern".to_string(),
                    src.span,
                ))
            }
        };

        if sources.is_empty() {
            return Err(ShellError::SpannedLabeledError(
                "No matches found".into(),
                "no matches found".into(),
                src.span,
            ));
        }

        if sources.len() > 1 && !destination.is_dir() {
            return Err(ShellError::SpannedLabeledError(
                "Destination must be a directory when copying multiple files".into(),
                "is not a directory".into(),
                dst.span,
            ));
        }

        let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));

        if any_source_is_dir && !recursive {
            return Err(ShellError::SpannedLabeledError(
                "Directories must be copied using \"--recursive\"".into(),
                "resolves to a directory (not copied)".into(),
                src.span,
            ));
        }

        for entry in sources.into_iter().flatten() {
            let mut sources = FileStructure::new();
            sources.walk_decorate(&entry, engine_state, stack)?;

            if entry.is_file() {
                let sources = sources.paths_applying_with(|(source_file, _depth_level)| {
                    if destination.is_dir() {
                        let mut dest = canonicalize_with(&dst.item, &path)?;
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
                        std::fs::copy(src, dst).map_err(|e| {
                            ShellError::SpannedLabeledError(e.to_string(), e.to_string(), call.head)
                        })?;
                    }
                }
            } else if entry.is_dir() {
                let destination = if !destination.exists() {
                    destination.clone()
                } else {
                    match entry.file_name() {
                        Some(name) => destination.join(name),
                        None => {
                            return Err(ShellError::SpannedLabeledError(
                                "Copy aborted. Not a valid path".into(),
                                "not a valid path".into(),
                                dst.span,
                            ))
                        }
                    }
                };

                std::fs::create_dir_all(&destination).map_err(|e| {
                    ShellError::SpannedLabeledError(e.to_string(), e.to_string(), dst.span)
                })?;

                let sources = sources.paths_applying_with(|(source_file, depth_level)| {
                    let mut dest = destination.clone();
                    let path = canonicalize_with(&source_file, &path)?;

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
                            ShellError::SpannedLabeledError(e.to_string(), e.to_string(), dst.span)
                        })?;
                    }

                    if s.is_file() {
                        std::fs::copy(&s, &d).map_err(|e| {
                            ShellError::SpannedLabeledError(e.to_string(), e.to_string(), call.head)
                        })?;
                    }
                }
            }
        }

        Ok(PipelineData::new(call.head))
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
        ]
    }

    //     let mut sources =
    //         nu_glob::glob(&source.to_string_lossy()).map_or_else(|_| Vec::new(), Iterator::collect);
    //     if sources.is_empty() {
    //         return Err(ShellError::FileNotFound(call.positional[0].span));
    //     }

    //     if sources.len() > 1 && !destination.is_dir() {
    //         return Err(ShellError::MoveNotPossible {
    //             source_message: "Can't move many files".to_string(),
    //             source_span: call.positional[0].span,
    //             destination_message: "into single file".to_string(),
    //             destination_span: call.positional[1].span,
    //         });
    //     }

    //     let any_source_is_dir = sources.iter().any(|f| matches!(f, Ok(f) if f.is_dir()));
    //     let recursive: bool = call.has_flag("recursive");
    //     if any_source_is_dir && !recursive {
    //         return Err(ShellError::MoveNotPossibleSingle(
    //             "Directories must be copied using \"--recursive\"".to_string(),
    //             call.positional[0].span,
    //         ));
    //     }

    //     if interactive && !force {
    //         let mut remove: Vec<usize> = vec![];
    //         for (index, file) in sources.iter().enumerate() {
    //             let prompt = format!(
    //                 "Are you shure that you want to copy {} to {}?",
    //                 file.as_ref()
    //                     .map_err(|err| ShellError::SpannedLabeledError(
    //                         "Reference error".into(),
    //                         err.to_string(),
    //                         call.head
    //                     ))?
    //                     .file_name()
    //                     .ok_or_else(|| ShellError::SpannedLabeledError(
    //                         "File name error".into(),
    //                         "Unable to get file name".into(),
    //                         call.head
    //                     ))?
    //                     .to_str()
    //                     .ok_or_else(|| ShellError::SpannedLabeledError(
    //                         "Unable to get str error".into(),
    //                         "Unable to convert to str file name".into(),
    //                         call.head
    //                     ))?,
    //                 destination
    //                     .file_name()
    //                     .ok_or_else(|| ShellError::SpannedLabeledError(
    //                         "File name error".into(),
    //                         "Unable to get file name".into(),
    //                         call.head
    //                     ))?
    //                     .to_str()
    //                     .ok_or_else(|| ShellError::SpannedLabeledError(
    //                         "Unable to get str error".into(),
    //                         "Unable to convert to str file name".into(),
    //                         call.head
    //                     ))?,
    //             );

    //             let input = get_interactive_confirmation(prompt)?;

    //             if !input {
    //                 remove.push(index);
    //             }
    //         }

    //         remove.reverse();

    //         for index in remove {
    //             sources.remove(index);
    //         }

    //         if sources.is_empty() {
    //             return Err(ShellError::NoFileToBeCopied());
    //         }
    //     }

    //     for entry in sources.into_iter().flatten() {
    //         let mut sources = FileStructure::new();
    //         sources.walk_decorate(&entry, engine_state, stack)?;

    //         if entry.is_file() {
    //             let sources = sources.paths_applying_with(|(source_file, _depth_level)| {
    //                 if destination.is_dir() {
    //                     let mut dest = canonicalize_with(&destination, &path)?;
    //                     if let Some(name) = entry.file_name() {
    //                         dest.push(name);
    //                     }
    //                     Ok((source_file, dest))
    //                 } else {
    //                     Ok((source_file, destination.clone()))
    //                 }
    //             })?;

    //             for (src, dst) in sources {
    //                 if src.is_file() {
    //                     std::fs::copy(&src, dst).map_err(|e| {
    //                         ShellError::MoveNotPossibleSingle(
    //                             format!(
    //                                 "failed to move containing file \"{}\": {}",
    //                                 src.to_string_lossy(),
    //                                 e
    //                             ),
    //                             call.positional[0].span,
    //                         )
    //                     })?;
    //                 }
    //             }
    //         } else if entry.is_dir() {
    //             let destination = if !destination.exists() {
    //                 destination.clone()
    //             } else {
    //                 match entry.file_name() {
    //                     Some(name) => destination.join(name),
    //                     None => {
    //                         return Err(ShellError::FileNotFoundCustom(
    //                             format!("containing \"{:?}\" is not a valid path", entry),
    //                             call.positional[0].span,
    //                         ))
    //                     }
    //                 }
    //             };

    //             std::fs::create_dir_all(&destination).map_err(|e| {
    //                 ShellError::MoveNotPossibleSingle(
    //                     format!("failed to recursively fill destination: {}", e),
    //                     call.positional[1].span,
    //                 )
    //             })?;

    //             let sources = sources.paths_applying_with(|(source_file, depth_level)| {
    //                 let mut dest = destination.clone();
    //                 let path = canonicalize_with(&source_file, &path)?;
    //                 let components = path
    //                     .components()
    //                     .map(|fragment| fragment.as_os_str())
    //                     .rev()
    //                     .take(1 + depth_level);

    //                 components.for_each(|fragment| dest.push(fragment));
    //                 Ok((PathBuf::from(&source_file), dest))
    //             })?;

    //             for (src, dst) in sources {
    //                 if src.is_dir() && !dst.exists() {
    //                     std::fs::create_dir_all(&dst).map_err(|e| {
    //                         ShellError::MoveNotPossibleSingle(
    //                             format!(
    //                                 "failed to create containing directory \"{}\": {}",
    //                                 dst.to_string_lossy(),
    //                                 e
    //                             ),
    //                             call.positional[1].span,
    //                         )
    //                     })?;
    //                 }

    //                 if src.is_file() {
    //                     std::fs::copy(&src, &dst).map_err(|e| {
    //                         ShellError::MoveNotPossibleSingle(
    //                             format!(
    //                                 "failed to move containing file \"{}\": {}",
    //                                 src.to_string_lossy(),
    //                                 e
    //                             ),
    //                             call.positional[0].span,
    //                         )
    //                     })?;
    //                 }
    //             }
    //         }
    //     }

    //     Ok(PipelineData::new(call.head))
    // }
}
