use filetime::FileTime;
use nu_engine::command_prelude::*;
use nu_path::expand_path_with;
use nu_protocol::NuGlob;
use std::{fs::OpenOptions, time::SystemTime};

#[derive(Clone)]
pub struct Touch;

impl Command for Touch {
    fn name(&self) -> &str {
        "touch"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["create", "file"]
    }

    fn signature(&self) -> Signature {
        Signature::build("touch")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "files",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Filepath]),
                "The file(s) to create."
            )
            .named(
                "reference",
                SyntaxShape::String,
                "change the file or directory time to the time of the reference file/directory",
                Some('r'),
            )
            .switch(
                "modified",
                "change the modification time of the file or directory. If no reference file/directory is given, the current time is used",
                Some('m'),
            )
            .switch(
                "access",
                "change the access time of the file or directory. If no reference file/directory is given, the current time is used",
                Some('a'),
            )
            .switch(
                "no-create",
                "do not create the file if it does not exist",
                Some('c'),
            )
            .switch(
                "no-deref",
                "do not follow symlinks",
                Some('s')
            )
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Creates one or more files."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut change_mtime: bool = call.has_flag(engine_state, stack, "modified")?;
        let mut change_atime: bool = call.has_flag(engine_state, stack, "access")?;
        let no_follow_symlinks: bool = call.has_flag(engine_state, stack, "no-deref")?;
        let reference: Option<Spanned<String>> = call.get_flag(engine_state, stack, "reference")?;
        let no_create: bool = call.has_flag(engine_state, stack, "no-create")?;
        let files = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;

        let cwd = engine_state.cwd(Some(stack))?;

        if files.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "requires file paths".to_string(),
                span: call.head,
            });
        }

        let mut mtime = SystemTime::now();
        let mut atime = mtime;

        // Change both times if neither is specified
        if !change_mtime && !change_atime {
            change_mtime = true;
            change_atime = true;
        }

        if let Some(reference) = reference {
            let reference_path = nu_path::expand_path_with(reference.item, &cwd, true);
            let exists = if no_follow_symlinks {
                // There's no symlink_exists function, so we settle for
                // getting direct metadata and if it's OK, it exists
                reference_path.symlink_metadata().is_ok()
            } else {
                reference_path.exists()
            };
            if !exists {
                return Err(ShellError::FileNotFoundCustom {
                    msg: "Reference path not found".into(),
                    span: reference.span,
                });
            }

            let metadata = if no_follow_symlinks {
                reference_path.symlink_metadata()
            } else {
                reference_path.metadata()
            };
            let metadata = metadata.map_err(|err| ShellError::IOErrorSpanned {
                msg: format!("Failed to read metadata: {err}"),
                span: reference.span,
            })?;
            mtime = metadata
                .modified()
                .map_err(|err| ShellError::IOErrorSpanned {
                    msg: format!("Failed to read modified time: {err}"),
                    span: reference.span,
                })?;
            atime = metadata
                .accessed()
                .map_err(|err| ShellError::IOErrorSpanned {
                    msg: format!("Failed to read access time: {err}"),
                    span: reference.span,
                })?;
        }

        for glob in files {
            let path = expand_path_with(glob.item.as_ref(), &cwd, glob.item.is_expand());
            let exists = if no_follow_symlinks {
                path.symlink_metadata().is_ok()
            } else {
                path.exists()
            };

            // If --no-create is passed and the file/dir does not exist there's nothing to do
            if no_create && !exists {
                continue;
            }

            // If --no-deref was passed in, the behavior of touch is to error on missing
            if no_follow_symlinks && !exists {
                return Err(ShellError::FileNotFound {
                    file: path.to_string_lossy().into_owned(),
                    span: glob.span,
                });
            }

            // Create a file at the given path unless the path is a directory (or a symlink with -d)
            if !path.is_dir() && (!no_follow_symlinks || !path.is_symlink()) {
                if let Err(err) = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path)
                {
                    return Err(ShellError::CreateNotPossible {
                        msg: format!("Failed to create file: {err}"),
                        span: glob.span,
                    });
                };
            }

            // We have to inefficiently access the target metadata to not reset it
            // in set_symlink_file_times, because the filetime doesn't expose individual methods for it
            let get_target_metadata = || {
                path.symlink_metadata()
                    .map_err(|err| ShellError::IOErrorSpanned {
                        msg: format!("Failed to read metadata: {err}"),
                        span: glob.span,
                    })
            };

            if change_mtime {
                let result = if no_follow_symlinks {
                    filetime::set_symlink_file_times(
                        &path,
                        if change_atime {
                            FileTime::from_system_time(atime)
                        } else {
                            FileTime::from_system_time(get_target_metadata()?.accessed()?)
                        },
                        FileTime::from_system_time(mtime),
                    )
                } else {
                    filetime::set_file_mtime(&path, FileTime::from_system_time(mtime))
                };
                if let Err(err) = result {
                    return Err(ShellError::ChangeModifiedTimeNotPossible {
                        msg: format!("Failed to change the modified time: {err}"),
                        span: glob.span,
                    });
                };
            }

            if change_atime {
                let result = if no_follow_symlinks {
                    filetime::set_symlink_file_times(
                        &path,
                        FileTime::from_system_time(atime),
                        if change_mtime {
                            FileTime::from_system_time(mtime)
                        } else {
                            FileTime::from_system_time(get_target_metadata()?.modified()?)
                        },
                    )
                } else {
                    filetime::set_file_atime(&path, FileTime::from_system_time(atime))
                };
                if let Err(err) = result {
                    return Err(ShellError::ChangeAccessTimeNotPossible {
                        msg: format!("Failed to change the access time: {err}"),
                        span: glob.span,
                    });
                };
            }
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates \"fixture.json\"",
                example: "touch fixture.json",
                result: None,
            },
            Example {
                description: "Creates files a, b and c",
                example: "touch a b c",
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of "fixture.json" to today's date"#,
                example: "touch -m fixture.json",
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of file d and e to "fixture.json"'s last modified time"#,
                example: r#"touch -m -r fixture.json d e"#,
                result: None,
            },
        ]
    }
}
