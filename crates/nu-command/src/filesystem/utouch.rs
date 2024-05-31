use std::io::ErrorKind;
use std::path::PathBuf;

use chrono::{DateTime, FixedOffset};
use filetime::FileTime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type,
};
use uu_touch::error::TouchError;
use uu_touch::{ChangeTimes, InputFile, Options, Source};

#[derive(Clone)]
pub struct UTouch;

impl Command for UTouch {
    fn name(&self) -> &str {
        "utouch"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["create", "file"]
    }

    fn signature(&self) -> Signature {
        Signature::build("utouch")
            .input_output_types(vec![ (Type::Nothing, Type::Nothing) ])
            .required(
                "filename",
                SyntaxShape::Filepath,
                "The path of the file you want to create.",
            )
            .named(
                "reference",
                SyntaxShape::Filepath,
                "change the file or directory time to the time of the reference file/directory",
                Some('r'),
            )
            .named(
                "timestamp",
                SyntaxShape::DateTime,
                "use the given time instead of the current time",
                Some('t')
            )
            .named(
                "date",
                SyntaxShape::String,
                "use the given date instead of the current date",
                Some('d')
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
            .switch(
                "no-dereference",
                "affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink)",
                None
            )
            .rest("rest", SyntaxShape::Filepath, "Additional files to create.")
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
        let change_mtime: bool = call.has_flag(engine_state, stack, "modified")?;
        let change_atime: bool = call.has_flag(engine_state, stack, "access")?;
        let no_create: bool = call.has_flag(engine_state, stack, "no-create")?;
        let no_deref: bool = call.has_flag(engine_state, stack, "no-dereference")?;
        let target: Spanned<String> = call.req(engine_state, stack, 0)?;
        let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 1)?;

        let (reference_file, reference_span) = if let Some(reference) =
            call.get_flag::<Spanned<PathBuf>>(engine_state, stack, "reference")?
        {
            (Some(reference.item), Some(reference.span))
        } else {
            (None, None)
        };
        let (date_str, date_span) =
            if let Some(date) = call.get_flag::<Spanned<String>>(engine_state, stack, "date")? {
                (Some(date.item), Some(date.span))
            } else {
                (None, None)
            };
        let timestamp: Option<Spanned<DateTime<FixedOffset>>> =
            call.get_flag(engine_state, stack, "timestamp")?;

        let source = if let Some(timestamp) = timestamp {
            if let Some(reference_span) = reference_span {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "timestamp given".to_string(),
                    left_span: timestamp.span,
                    right_message: "reference given".to_string(),
                    right_span: reference_span,
                });
            }
            if let Some(date_span) = date_span {
                return Err(ShellError::IncompatibleParameters {
                    left_message: "timestamp given".to_string(),
                    left_span: timestamp.span,
                    right_message: "date given".to_string(),
                    right_span: date_span,
                });
            }
            Source::Timestamp(FileTime::from_unix_time(
                timestamp.item.timestamp(),
                timestamp.item.timestamp_subsec_nanos(),
            ))
        } else if let Some(reference_file) = reference_file {
            Source::Reference(reference_file)
        } else {
            Source::Now
        };

        let change_times = if change_atime && !change_mtime {
            ChangeTimes::AtimeOnly
        } else if change_mtime && !change_atime {
            ChangeTimes::MtimeOnly
        } else {
            ChangeTimes::Both
        };

        let mut files = vec![InputFile::Path(PathBuf::from(target.item))];
        let mut file_spans = vec![target.span];
        for file in rest {
            files.push(InputFile::Path(PathBuf::from(file.item)));
            file_spans.push(file.span);
        }

        if let Err(err) = uu_touch::touch(
            &files,
            &Options {
                no_create,
                no_deref,
                source,
                date: date_str,
                change_times,
                strict: true,
            },
        ) {
            let nu_err = match err {
                TouchError::InvalidDateFormat(date) => ShellError::IncorrectValue {
                    msg: format!("Invalid date: {}", date),
                    val_span: date_span.expect("utouch was given a date"),
                    call_span: call.head,
                },
                TouchError::ReferenceFileInaccessible(reference_path, io_err) => {
                    let span = reference_span.expect("utouch was given a reference file");
                    if io_err.kind() == ErrorKind::NotFound {
                        // todo merge main into this to say which file not found
                        ShellError::FileNotFound { span }
                    } else {
                        io_to_nu_err(
                            io_err,
                            format!("Failed to read metadata of {}", reference_path.display()),
                            span,
                        )
                    }
                }
                _ => ShellError::GenericError {
                    error: err.to_string(),
                    msg: err.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: Vec::new(),
                },
            };
            return Err(nu_err);
        }

        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Creates \"fixture.json\"",
                example: "utouch fixture.json",
                result: None,
            },
            Example {
                description: "Creates files a, b and c",
                example: "utouch a b c",
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of "fixture.json" to today's date"#,
                example: "utouch -m fixture.json",
                result: None,
            },
            Example {
                description: "Changes the last modified time of files a, b and c to the current time but yesterday",
                example: r#"utouch -m -d "yesterday" a b c"#,
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of file d and e to "fixture.json"'s last modified time"#,
                example: r#"utouch -m -r fixture.json d e"#,
                result: None,
            },
            Example {
                description: r#"Changes the last accessed time of "fixture.json" to a datetime"#,
                example: r#"utouch -a -t 2019-08-24T12:30:30 fixture.json"#,
                result: None,
            },
        ]
    }
}

fn io_to_nu_err(err: std::io::Error, msg: String, span: Span) -> ShellError {
    if err.kind() == ErrorKind::PermissionDenied {
        ShellError::PermissionDeniedError { msg, span }
    } else {
        ShellError::GenericError {
            error: err.to_string(),
            msg,
            span: Some(span),
            help: None,
            inner: Vec::new(),
        }
    }
}
