use chrono::{DateTime, FixedOffset};
use filetime::FileTime;
use nu_engine::command_prelude::*;
use nu_glob::{glob, is_glob};
use nu_path::expand_path_with;
use nu_protocol::{NuGlob, shell_error::io::IoError};
use std::path::PathBuf;
use uu_touch::{ChangeTimes, InputFile, Options, Source, error::TouchError};

#[derive(Clone)]
pub struct UTouch;

impl Command for UTouch {
    fn name(&self) -> &str {
        "touch"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["create", "file", "coreutils"]
    }

    fn signature(&self) -> Signature {
        Signature::build("touch")
            .input_output_types(vec![ (Type::Nothing, Type::Nothing) ])
            .rest(
                "files",
                SyntaxShape::OneOf(vec![SyntaxShape::GlobPattern, SyntaxShape::Filepath]),
                "The file(s) to create. '-' is used to represent stdout."
            )
            .named(
                "reference",
                SyntaxShape::Filepath,
                "Use the access and modification times of the reference file/directory instead of the current time",
                Some('r'),
            )
            .named(
                "timestamp",
                SyntaxShape::DateTime,
                "Use the given timestamp instead of the current time",
                Some('t')
            )
            .named(
                "date",
                SyntaxShape::String,
                "Use the given time instead of the current time. This can be a full timestamp or it can be relative to either the current time or reference file time (if given). For more information, see https://www.gnu.org/software/coreutils/manual/html_node/touch-invocation.html",
                Some('d')
            )
            .switch(
                "modified",
                "Change only the modification time (if used with -a, access time is changed too)",
                Some('m'),
            )
            .switch(
                "access",
                "Change only the access time (if used with -m, modification time is changed too)",
                Some('a'),
            )
            .switch(
                "no-create",
                "Don't create the file if it doesn't exist",
                Some('c'),
            )
            .switch(
                "no-deref",
                "Affect each symbolic link instead of any referenced file (only for systems that can change the timestamps of a symlink). Ignored if touching stdout",
                Some('s'),
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
        let change_mtime: bool = call.has_flag(engine_state, stack, "modified")?;
        let change_atime: bool = call.has_flag(engine_state, stack, "access")?;
        let no_create: bool = call.has_flag(engine_state, stack, "no-create")?;
        let no_deref: bool = call.has_flag(engine_state, stack, "no-deref")?;
        let file_globs = call.rest::<Spanned<NuGlob>>(engine_state, stack, 0)?;
        let cwd = engine_state.cwd(Some(stack))?;

        if file_globs.is_empty() {
            return Err(ShellError::MissingParameter {
                param_name: "requires file paths".to_string(),
                span: call.head,
            });
        }

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
            let reference_file = expand_path_with(reference_file, &cwd, true);
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

        let mut input_files = Vec::new();
        for file_glob in &file_globs {
            if file_glob.item.as_ref() == "-" {
                input_files.push(InputFile::Stdout);
            } else {
                let file_path =
                    expand_path_with(file_glob.item.as_ref(), &cwd, file_glob.item.is_expand());

                if !file_glob.item.is_expand() {
                    input_files.push(InputFile::Path(file_path));
                    continue;
                }

                let mut expanded_globs =
                    glob(&file_path.to_string_lossy(), engine_state.signals().clone())
                        .unwrap_or_else(|_| {
                            panic!(
                                "Failed to process file path: {}",
                                &file_path.to_string_lossy()
                            )
                        })
                        .peekable();

                if expanded_globs.peek().is_none() {
                    let file_name = file_path.file_name().unwrap_or_else(|| {
                        panic!(
                            "Failed to process file path: {}",
                            &file_path.to_string_lossy()
                        )
                    });

                    if is_glob(&file_name.to_string_lossy()) {
                        return Err(ShellError::GenericError {
                            error: format!(
                                "No matches found for glob {}",
                                file_name.to_string_lossy()
                            ),
                            msg: "No matches found for glob".into(),
                            span: Some(file_glob.span),
                            help: Some(format!(
                                "Use quotes if you want to create a file named {}",
                                file_name.to_string_lossy()
                            )),
                            inner: vec![],
                        });
                    }

                    input_files.push(InputFile::Path(file_path));
                    continue;
                }

                input_files.extend(expanded_globs.filter_map(Result::ok).map(InputFile::Path));
            }
        }

        if let Err(err) = uu_touch::touch(
            &input_files,
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
                TouchError::TouchFileError { path, index, error } => ShellError::GenericError {
                    error: format!("Could not touch {}", path.display()),
                    msg: error.to_string(),
                    span: Some(file_globs[index].span),
                    help: None,
                    inner: Vec::new(),
                },
                TouchError::InvalidDateFormat(date) => ShellError::IncorrectValue {
                    msg: format!("Invalid date: {date}"),
                    val_span: date_span.expect("touch should've been given a date"),
                    call_span: call.head,
                },
                TouchError::ReferenceFileInaccessible(reference_path, io_err) => {
                    let span = reference_span.expect("touch should've been given a reference file");
                    ShellError::Io(IoError::new_with_additional_context(
                        io_err,
                        span,
                        reference_path,
                        "failed to read metadata",
                    ))
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

    fn examples(&self) -> Vec<Example<'_>> {
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
                description: r#"Changes the last modified and accessed time of all files with the .json extension to today's date"#,
                example: "touch *.json",
                result: None,
            },
            Example {
                description: "Changes the last accessed and modified times of files a, b and c to the current time but yesterday",
                example: r#"touch -d "yesterday" a b c"#,
                result: None,
            },
            Example {
                description: r#"Changes the last modified time of files d and e to "fixture.json"'s last modified time"#,
                example: r#"touch -m -r fixture.json d e"#,
                result: None,
            },
            Example {
                description: r#"Changes the last accessed time of "fixture.json" to a datetime"#,
                example: r#"touch -a -t 2019-08-24T12:30:30 fixture.json"#,
                result: None,
            },
            Example {
                description: r#"Change the last accessed and modified times of stdout"#,
                example: r#"touch -"#,
                result: None,
            },
            Example {
                description: r#"Changes the last accessed and modified times of file a to 1 month before "fixture.json"'s last modified time"#,
                example: r#"touch -r fixture.json -d "-1 month" a"#,
                result: None,
            },
        ]
    }
}
