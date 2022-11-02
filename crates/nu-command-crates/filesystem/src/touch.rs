use std::fs::OpenOptions;
use std::path::Path;

use chrono::{DateTime, Local};
use filetime::FileTime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

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
            .required(
                "filename",
                SyntaxShape::Filepath,
                "the path of the file you want to create",
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
            .rest("rest", SyntaxShape::Filepath, "additional files to create")
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
        let mut change_mtime: bool = call.has_flag("modified");
        let mut change_atime: bool = call.has_flag("access");
        let use_reference: bool = call.has_flag("reference");
        let no_create: bool = call.has_flag("no-create");
        let target: String = call.req(engine_state, stack, 0)?;
        let rest: Vec<String> = call.rest(engine_state, stack, 1)?;

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
                        return Err(ShellError::UnsupportedInput(
                            "path provided is invalid".to_string(),
                            reference.span,
                        ));
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
                    return Err(ShellError::MissingParameter(
                        "reference".to_string(),
                        call.head,
                    ));
                }
            }
        }

        for (index, item) in vec![target].into_iter().chain(rest).enumerate() {
            if no_create {
                let path = Path::new(&item);
                if !path.exists() {
                    continue;
                }
            }

            if let Err(err) = OpenOptions::new().write(true).create(true).open(&item) {
                return Err(ShellError::CreateNotPossible(
                    format!("Failed to create file: {}", err),
                    call.positional_nth(index)
                        .expect("already checked positional")
                        .span,
                ));
            };

            if change_mtime {
                // Should not panic as we return an error above if we can't parse the date
                if let Err(err) = filetime::set_file_mtime(
                    &item,
                    FileTime::from_system_time(date.expect("should be a valid date").into()),
                ) {
                    return Err(ShellError::ChangeModifiedTimeNotPossible(
                        format!("Failed to change the modified time: {}", err),
                        call.positional_nth(index)
                            .expect("already checked positional")
                            .span,
                    ));
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
                        return Err(ShellError::ChangeAccessTimeNotPossible(
                            format!("Failed to change the access time: {}", err),
                            call.positional_nth(index)
                                .expect("already checked positional")
                                .span,
                        ));
                    };
                } else {
                    // Should not panic as we return an error above if we can't parse the date
                    if let Err(err) = filetime::set_file_atime(
                        &item,
                        FileTime::from_system_time(date.expect("should be a valid date").into()),
                    ) {
                        return Err(ShellError::ChangeAccessTimeNotPossible(
                            format!("Failed to change the access time: {}", err),
                            call.positional_nth(index)
                                .expect("already checked positional")
                                .span,
                        ));
                    };
                }
            }
        }

        Ok(PipelineData::new(call.head))
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
