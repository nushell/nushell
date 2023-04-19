use std::fs::OpenOptions;
use std::path::Path;

use chrono::{DateTime, Local, Timelike};
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
            .input_output_types(vec![ (Type::Nothing, Type::Nothing) ])
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
            .named(
                "date",
                SyntaxShape::String,
                "change the file or directory time to a date",
                Some('d')
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
        let date_flag = call.has_flag("date");
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

        if date_flag {
            let date_string: Option<Spanned<String>> =
                call.get_flag(engine_state, stack, "date")?;
            match date_string {
                Some(date_string) => {
                    // try to parse a relative date
                    let parsed_date = crate::util::parse_relative_time(&date_string.item);
                    if let Some(parsed_date) = parsed_date {
                        date = Some(Local::now() + parsed_date);
                    } else {
                        date = parse_given_string_to_date(date_string)?
                    }
                }
                None => {
                    return Err(ShellError::MissingParameter {
                        param_name: "date".to_string(),
                        span: call.head,
                    });
                }
            };
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

        for (index, item) in vec![target].into_iter().chain(rest).enumerate() {
            if no_create {
                let path = Path::new(&item);
                if !path.exists() {
                    continue;
                }
            }

            if let Err(err) = OpenOptions::new().write(true).create(true).open(&item) {
                return Err(ShellError::CreateNotPossible(
                    format!("Failed to create file: {err}"),
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
                        format!("Failed to change the modified time: {err}"),
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
                            format!("Failed to change the access time: {err}"),
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
                            format!("Failed to change the access time: {err}"),
                            call.positional_nth(index)
                                .expect("already checked positional")
                                .span,
                        ));
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
                description: r#"Changes the last modified time of "fixture.json" to today's date (local timezone)"#,
                example: "touch -m fixture.json",
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of "fixture.json" to now (local timezone)"#,
                example: "touch -m -d '' fixture.json",
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of "fixture.json" to now (local timezone)"#,
                example: "touch -m -d 'now' fixture.json",
                result: None,
            },
            Example {
                description:
                    "Changes the last modified time of files a, b and c to a date (keeps the file's time and timezone)",
                example: r#"touch -m -d "2015-09-13" a b c"#,
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of file d and e to "fixture.json"'s last modified time"#,
                example: r#"touch -m -r fixture.json d e"#,
                result: None,
            },
            Example {
                description: r#"Changes the last accessed time of "fixture.json" to a date" (keeps the file's timezone)"#,
                example: r#"touch -a -d "August 24, 2019 12:30:30" fixture.json"#,
                result: None,
            },
            Example {
                description: r#"Use relative time (seconds, minutes, hours, days, weeks) to change the last accessed time of "fixture.json" to a date. You can specify 2 hours ago, or -2 hours 
                for going back in time, or 2 hours or +2 hours for going forward in time"#,
                example: r#"touch -a -d "2 hours ago" fixture.json"#,
                result: None,
            },
        ]
    }
}

fn parse_given_string_to_date(
    date_string: Spanned<String>,
) -> Result<Option<DateTime<Local>>, ShellError> {
    match date_string.item.is_empty() {
        true => {
            let date = Local::now()
                .with_hour(0)
                .and_then(|x| x.with_minute(0))
                .and_then(|s| s.with_second(0));
            if let Some(date) = date {
                Ok(Some(date))
            } else {
                Err(ShellError::NushellFailed {
                    msg: "Cannot create a local now time".to_string(),
                })
            }
        }
        false if date_string.item == "." => Err(ShellError::DatetimeParseError(
            "Cannot parse the '.' date. Did you simply mean '' ?".to_string(),
            date_string.span,
        )),
        false => {
            let parsed_date = dtparse::parse(&date_string.item).ok();
            if let Some(date) = parsed_date {
                if let Some(offset) = date.1 {
                    Ok(Some(
                        DateTime::<Local>::from_local(date.0, offset).with_timezone(&Local),
                    ))
                } else {
                    Ok(Some(
                        // no offset, then use the local timezone
                        DateTime::<Local>::from_local(date.0, *Local::now().offset()),
                    ))
                }
            } else {
                Err(ShellError::DatetimeParseError(
                    "Cannot parse the provided date.".to_string(),
                    date_string.span,
                ))
            }
        }
    }
}
