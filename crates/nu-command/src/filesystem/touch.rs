use std::fs::OpenOptions;

use chrono::{offset::Utc, DateTime, Datelike, FixedOffset};
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
                "change the file or directory last modified time to a timestamp. Format: [[CC]YY]MMDDhhmm[.ss]",
                Some('t'),
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
        let change_stamp: bool = call.has_flag("timestamp");
        let target: String = call.req(engine_state, stack, 0)?;
        let rest: Vec<String> = call.rest(engine_state, stack, 1)?;

        let mut date: Option<DateTime<FixedOffset>> = None;

        if change_stamp {
            let stamp: Option<Spanned<String>> = call.get_flag(engine_state, stack, "timestamp")?;
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
            if (size % 2 != 0 || !(8..=14).contains(&size)) || val.parse::<usize>().is_err() {
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

            let val: String = if let Some(add_year) = add_year {
                let year = Utc::now().year();
                match add_year {
                    AddYear::Full => format!("{}{}", year, val),
                    AddYear::FirstDigits => format!("{}{}", year / 100, val),
                }
            } else {
                val
            };

            date = if let Ok(date) = parse_date_from_string(&val, span) {
                Some(date)
            } else {
                return Err(ShellError::UnsupportedInput(
                    "input has an invalid timestamp".to_string(),
                    span,
                ));
            };
        }

        for (index, item) in vec![target].into_iter().chain(rest).enumerate() {
            if let Err(err) = OpenOptions::new().write(true).create(true).open(&item) {
                return Err(ShellError::CreateNotPossible(
                    format!("Failed to create file: {}", err),
                    call.positional[index].span,
                ));
            };

            if change_stamp {
                // Safe to unwrap as we return an error above if we can't parse the date
                match filetime::set_file_mtime(
                    &item,
                    FileTime::from_system_time(date.unwrap().into()),
                ) {
                    Ok(_) => continue,
                    Err(err) => {
                        return Err(ShellError::ChangeModifiedTimeNotPossible(
                            format!("Failed to change the modified time: {}", err),
                            call.positional[index].span,
                        ));
                    }
                };
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
                description: "Creates files a, b and c with a timestamp",
                example: "touch -t 201908241230.30 a b c",
                result: None,
            },
        ]
    }
}
