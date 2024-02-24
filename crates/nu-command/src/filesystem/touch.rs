use std::fs::OpenOptions;
use std::path::Path;

use chrono::{DateTime, Local};
use filetime::FileTime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape, Type,
};

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
            .rest("files", SyntaxShape::Filepath, "The file(s) to create.")
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
        let use_reference: bool = call.has_flag(engine_state, stack, "reference")?;
        let no_create: bool = call.has_flag(engine_state, stack, "no-create")?;
        let files: Vec<String> = call.rest(engine_state, stack, 0)?;

        if files.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "requires file paths".to_string(),
                span: call.head,
            });
        }

        let mut date: Option<DateTime<Local>> = None;
        let mut ref_date_atime: Option<DateTime<Local>> = None;

        // Change both times if none is specified
        if !change_mtime && !change_atime {
            change_mtime = true;
            change_atime = true;
        }

        if change_mtime || change_atime {
            date = Some(Local::now());
        }

        if use_reference {
            let reference: Option<Spanned<String>> =
                call.get_flag(engine_state, stack, "reference")?;
            match reference {
                Some(reference) => {
                    let reference_path = Path::new(&reference.item);
                    if !reference_path.exists() {
                        return Err(ShellError::TypeMismatch {
                            err_message: "path provided is invalid".to_string(),
                            span: reference.span,
                        });
                    }

                    date = Some(
                        reference_path
                            .metadata()
                            .expect("should be a valid path") // Should never fail as the path exists
                            .modified()
                            .expect("should have metadata") // This should always be valid as it is available on all nushell's supported platforms (Linux, Windows, MacOS)
                            .into(),
                    );

                    ref_date_atime = Some(
                        reference_path
                            .metadata()
                            .expect("should be a valid path") // Should never fail as the path exists
                            .accessed()
                            .expect("should have metadata") // This should always be valid as it is available on all nushell's supported platforms (Linux, Windows, MacOS)
                            .into(),
                    );
                }
                None => {
                    return Err(ShellError::MissingParameter {
                        param_name: "reference".to_string(),
                        span: call.head,
                    });
                }
            }
        }

        for (index, item) in files.into_iter().enumerate() {
            if no_create {
                let path = Path::new(&item);
                if !path.exists() {
                    continue;
                }
            }

            if let Err(err) = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&item)
            {
                return Err(ShellError::CreateNotPossible {
                    msg: format!("Failed to create file: {err}"),
                    span: call
                        .positional_nth(index)
                        .expect("already checked positional")
                        .span,
                });
            };

            if change_mtime {
                // Should not panic as we return an error above if we can't parse the date
                if let Err(err) = filetime::set_file_mtime(
                    &item,
                    FileTime::from_system_time(date.expect("should be a valid date").into()),
                ) {
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
                // Reference file/directory may have different access and modified times
                if use_reference {
                    // Should not panic as we return an error above if we can't parse the date
                    if let Err(err) = filetime::set_file_atime(
                        &item,
                        FileTime::from_system_time(
                            ref_date_atime.expect("should be a valid date").into(),
                        ),
                    ) {
                        return Err(ShellError::ChangeAccessTimeNotPossible {
                            msg: format!("Failed to change the access time: {err}"),
                            span: call
                                .positional_nth(index)
                                .expect("already checked positional")
                                .span,
                        });
                    };
                } else {
                    // Should not panic as we return an error above if we can't parse the date
                    if let Err(err) = filetime::set_file_atime(
                        &item,
                        FileTime::from_system_time(date.expect("should be a valid date").into()),
                    ) {
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
