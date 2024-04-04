use filetime::FileTime;
use nu_engine::{command_prelude::*, current_dir};
use nu_path::expand_path_with;
use nu_protocol::NuGlob;

use std::{fs::OpenOptions, path::Path, time::SystemTime};

use super::util::get_rest_for_glob_pattern;

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
                "change the modification time of the file or directory. If no timestamp, date or reference file/directory is given, the current time is used",
                Some('m'),
            )
            .switch(
                "access",
                "change the access time of the file or directory. If no timestamp, date or reference file/directory is given, the current time is used",
                Some('a'),
            )
            .switch(
                "no-create",
                "do not create the file if it does not exist",
                Some('c'),
            )
            .category(Category::FileSystem)
    }

    fn usage(&self) -> &str {
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
        let reference: Option<Spanned<String>> = call.get_flag(engine_state, stack, "reference")?;
        let no_create: bool = call.has_flag(engine_state, stack, "no-create")?;
        let files: Vec<Spanned<NuGlob>> = get_rest_for_glob_pattern(engine_state, stack, call, 0)?;

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
            let reference_path = Path::new(&reference.item);
            if !reference_path.exists() {
                return Err(ShellError::FileNotFoundCustom {
                    msg: "Reference path not found".into(),
                    span: reference.span,
                });
            }

            let metadata = reference_path
                .metadata()
                .map_err(|err| ShellError::IOErrorSpanned {
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

        let cwd = current_dir(engine_state, stack)?;

        for (index, glob) in files.into_iter().enumerate() {
            let path = expand_path_with(glob.item.as_ref(), &cwd, glob.item.is_expand());

            // If --no-create is passed and the file/dir does not exist there's nothing to do
            if no_create && !path.exists() {
                continue;
            }

            // Create a file at the given path unless the path is a directory
            if !path.is_dir() {
                if let Err(err) = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&path)
                {
                    return Err(ShellError::CreateNotPossible {
                        msg: format!("Failed to create file: {err}"),
                        span: call
                            .positional_nth(index)
                            .expect("already checked positional")
                            .span,
                    });
                };
            }

            if change_mtime {
                if let Err(err) = filetime::set_file_mtime(&path, FileTime::from_system_time(mtime))
                {
                    return Err(ShellError::ChangeModifiedTimeNotPossible {
                        msg: format!("Failed to change the modified time: {err}"),
                        span: call
                            .positional_nth(index)
                            .expect("already checked positional")
                            .span,
                    });
                };
            }

            if change_atime {
                if let Err(err) = filetime::set_file_atime(&path, FileTime::from_system_time(atime))
                {
                    return Err(ShellError::ChangeAccessTimeNotPossible {
                        msg: format!("Failed to change the access time: {err}"),
                        span: call
                            .positional_nth(index)
                            .expect("already checked positional")
                            .span,
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
                description: "Changes the last modified time of files a, b and c to a date",
                example: r#"touch -m -d "yesterday" a b c"#,
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of file d and e to "fixture.json"'s last modified time"#,
                example: r#"touch -m -r fixture.json d e"#,
                result: None,
            },
            Example {
                description: r#"Changes the last accessed time of "fixture.json" to a date"#,
                example: r#"touch -a -d "August 24, 2019; 12:30:30" fixture.json"#,
                result: None,
            },
        ]
    }
}
