use std::fs::OpenOptions;
use std::path::Path;

use chrono::{DateTime, Datelike, Local};
use filetime::FileTime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, Spanned, SyntaxShape};

use crate::parse_date_from_string;

enum AddYear {
    Full,
    FirstDigits,
}

#[derive(Clone)]
pub struct Touch;

impl Command for Touch {
    fn name(&self) -> &str {
        "touch"
    }

    fn signature(&self) -> Signature {
        Signature::build("touch")
            .required(
                "filename",
                SyntaxShape::Filepath,
                "the path of the file you want to create",
            )
            .named(
                "timestamp",
                SyntaxShape::String,
                "change the file or directory time to a timestamp. Format: [[CC]YY]MMDDhhmm[.ss]\n\n      If neither YY or CC is given, the current year will be assumed. If YY is specified, but CC is not, CC will be derived as follows:\n      \tIf YY is between [69, 99], CC is 19\n      \tIf YY is between [00, 68], CC is 20\n      Note: It is expected that in a future version of this standard the default century inferred from a 2-digit year will change",
                Some('t'),
            )
            .named(
                "date",
                SyntaxShape::String,
                "change the file or directory time to a date",
                Some('d'),
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
        let use_stamp: bool = call.has_flag("timestamp");
        let use_date: bool = call.has_flag("date");
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

        if use_stamp || use_date {
            let (val, span) = if use_stamp {
                let stamp: Option<Spanned<String>> =
                    call.get_flag(engine_state, stack, "timestamp")?;
                let (stamp, span) = match stamp {
                    Some(stamp) => (stamp.item, stamp.span),
                    None => {
                        return Err(ShellError::MissingParameter(
                            "timestamp".to_string(),
                            call.head,
                        ));
                    }
                };

                // Checks for the seconds stamp and removes the '.' delimiter if any
                let (val, has_sec): (String, bool) = match stamp.split_once('.') {
                    Some((dtime, sec)) => (format!("{}{}", dtime, sec), true),
                    None => (stamp.to_string(), false),
                };

                let size = val.len();

                // Each stamp is a 2 digit number and the whole stamp must not be less than 4 or greater than 7 pairs
                if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<u64>().is_err() {
                    return Err(ShellError::UnsupportedInput(
                        "input has an invalid timestamp".to_string(),
                        span,
                    ));
                }

                let add_year: Option<AddYear> = if has_sec {
                    match size {
                        10 => Some(AddYear::Full),
                        12 => Some(AddYear::FirstDigits),
                        14 => None,
                        _ => unreachable!(), // This should never happen as the check above should catch it
                    }
                } else {
                    match size {
                        8 => Some(AddYear::Full),
                        10 => Some(AddYear::FirstDigits),
                        12 => None,
                        _ => unreachable!(), // This should never happen as the check above should catch it
                    }
                };

                if let Some(add_year) = add_year {
                    let year = Local::now().year();
                    match add_year {
                        AddYear::Full => (format!("{}{}", year, val), span),
                        AddYear::FirstDigits => {
                            // Compliance with the Unix version of touch
                            let yy = val[0..2]
                                .parse::<u8>()
                                .expect("should be a valid 2 digit number");
                            let mut year = 20;
                            if (69..=99).contains(&yy) {
                                year = 19;
                            }
                            (format!("{}{}", year, val), span)
                        }
                    }
                } else {
                    (val, span)
                }
            } else {
                let date_string: Option<Spanned<String>> =
                    call.get_flag(engine_state, stack, "date")?;
                match date_string {
                    Some(date_string) => (date_string.item, date_string.span),
                    None => {
                        return Err(ShellError::MissingParameter("date".to_string(), call.head));
                    }
                }
            };

            date = if let Ok(parsed_date) = parse_date_from_string(&val, span) {
                Some(parsed_date.into())
            } else {
                let flag = if use_stamp { "timestamp" } else { "date" };
                return Err(ShellError::UnsupportedInput(
                    format!("input has an invalid {}", flag),
                    span,
                ));
            };
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
                description: "Creates files d and e and set its last modified time to a timestamp",
                example: "touch -m -t 201908241230.30 d e",
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
            Example {
                description: "Changes both last modified and accessed time of a, b and c to a timestamp only if they exist",
                example: r#"touch -c -t 201908241230.30 a b c"#,
                result: None,
            },
        ]
    }
}
