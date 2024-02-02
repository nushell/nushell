use nu_engine::{find_in_dirs_env, get_dirs_var_from_call, CallExt};
use nu_parser::{parse, parse_module_block, unescape_unquote_string};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct NuCheck;

impl Command for NuCheck {
    fn name(&self) -> &str {
        "nu-check"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu-check")
            .input_output_types(vec![(Type::String, Type::Bool),
            (Type::ListStream, Type::Bool),
            (Type::List(Box::new(Type::Any)), Type::Bool)])
            // type is string to avoid automatically canonicalizing the path
            .optional("path", SyntaxShape::String, "File path to parse.")
            .switch("as-module", "Parse content as module", Some('m'))
            .switch("debug", "Show error messages", Some('d'))
            .switch("all", "Parse content as script first, returns result if success, otherwise, try with module", Some('a'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Validate and parse input content."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["syntax", "parse", "debug"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        let is_module = call.has_flag(engine_state, stack, "as-module")?;
        let is_debug = call.has_flag(engine_state, stack, "debug")?;
        let is_all = call.has_flag(engine_state, stack, "all")?;
        let config = engine_state.get_config();
        let mut contents = vec![];

        // DO NOT ever try to merge the working_set in this command
        let mut working_set = StateWorkingSet::new(engine_state);

        if is_all && is_module {
            return Err(ShellError::GenericError {
                error: "Detected command flags conflict".into(),
                msg: "You cannot have both `--all` and `--as-module` on the same command line, please refer to `nu-check --help` for more details".into(),
                span: Some(call.head),
                help: None,
                inner: vec![]
            });
        }

        let span = input.span().unwrap_or(call.head);
        match input {
            PipelineData::Value(Value::String { val, .. }, ..) => {
                let contents = Vec::from(val);
                if is_all {
                    heuristic_parse(&mut working_set, None, &contents, is_debug, call.head)
                } else if is_module {
                    parse_module(&mut working_set, None, &contents, is_debug, span)
                } else {
                    parse_script(&mut working_set, None, &contents, is_debug, span)
                }
            }
            PipelineData::ListStream(stream, ..) => {
                let list_stream = stream.into_string("\n", config);
                let contents = Vec::from(list_stream);

                if is_all {
                    heuristic_parse(&mut working_set, None, &contents, is_debug, call.head)
                } else if is_module {
                    parse_module(&mut working_set, None, &contents, is_debug, call.head)
                } else {
                    parse_script(&mut working_set, None, &contents, is_debug, call.head)
                }
            }
            PipelineData::ExternalStream {
                stdout: Some(stream),
                ..
            } => {
                let raw_stream: Vec<_> = stream.stream.collect();
                for r in raw_stream {
                    match r {
                        Ok(v) => contents.extend(v),
                        Err(error) => return Err(error),
                    };
                }

                if is_all {
                    heuristic_parse(&mut working_set, None, &contents, is_debug, call.head)
                } else if is_module {
                    parse_module(&mut working_set, None, &contents, is_debug, call.head)
                } else {
                    parse_script(&mut working_set, None, &contents, is_debug, call.head)
                }
            }
            _ => {
                if let Some(path_str) = path {
                    // look up the path as relative to FILE_PWD or inside NU_LIB_DIRS (same process as source-env)
                    let path = match find_in_dirs_env(
                        &path_str.item,
                        engine_state,
                        stack,
                        get_dirs_var_from_call(call),
                    ) {
                        Ok(path) => {
                            if let Some(path) = path {
                                path
                            } else {
                                return Err(ShellError::FileNotFound {
                                    span: path_str.span,
                                });
                            }
                        }
                        Err(error) => return Err(error),
                    };

                    // get the expanded path as a string
                    let path_str = path.to_string_lossy().to_string();

                    let ext: Vec<_> = path_str.rsplitn(2, '.').collect();
                    if ext[0] != "nu" {
                        return Err(ShellError::GenericError {
                            error: "Cannot parse input".into(),
                            msg: "File extension must be the type of .nu".into(),
                            span: Some(call.head),
                            help: None,
                            inner: vec![],
                        });
                    }

                    // Change currently parsed directory
                    let prev_currently_parsed_cwd = if let Some(parent) = path.parent() {
                        let prev = working_set.currently_parsed_cwd.clone();

                        working_set.currently_parsed_cwd = Some(parent.into());

                        prev
                    } else {
                        working_set.currently_parsed_cwd.clone()
                    };

                    let result = if is_all {
                        heuristic_parse_file(path_str, &mut working_set, call, is_debug)
                    } else if is_module {
                        parse_file_module(path_str, &mut working_set, call, is_debug)
                    } else {
                        parse_file_script(path_str, &mut working_set, call, is_debug)
                    };

                    // Restore the currently parsed directory back
                    working_set.currently_parsed_cwd = prev_currently_parsed_cwd;

                    result
                } else {
                    Err(ShellError::GenericError {
                        error: "Failed to execute command".into(),
                        msg: "Please run 'nu-check --help' for more details".into(),
                        span: Some(call.head),
                        help: None,
                        inner: vec![],
                    })
                }
            }
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse a input file as script(Default)",
                example: "nu-check script.nu",
                result: None,
            },
            Example {
                description: "Parse a input file as module",
                example: "nu-check --as-module module.nu",
                result: None,
            },
            Example {
                description: "Parse a input file by showing error message",
                example: "nu-check --debug script.nu",
                result: None,
            },
            Example {
                description: "Parse an external stream as script by showing error message",
                example: "open foo.nu | nu-check --debug script.nu",
                result: None,
            },
            Example {
                description: "Parse an internal stream as module by showing error message",
                example: "open module.nu | lines | nu-check --debug --as-module module.nu",
                result: None,
            },
            Example {
                description: "Parse a string as script",
                example: "$'two(char nl)lines' | nu-check ",
                result: None,
            },
            Example {
                description: "Heuristically parse which begins with script first, if it sees a failure, try module afterwards",
                example: "nu-check -a script.nu",
                result: None,
            },
            Example {
                description: "Heuristically parse by showing error message",
                example: "open foo.nu | lines | nu-check --all --debug",
                result: None,
            },
        ]
    }
}

fn heuristic_parse(
    working_set: &mut StateWorkingSet,
    filename: Option<&str>,
    contents: &[u8],
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    match parse_script(working_set, filename, contents, is_debug, span) {
        Ok(v) => Ok(v),
        Err(_) => {
            match parse_module(
                working_set,
                filename.map(|f| f.to_string()),
                contents,
                is_debug,
                span,
            ) {
                Ok(v) => Ok(v),
                Err(_) => {
                    if is_debug {
                        Err(ShellError::GenericError {
                            error: "Failed to parse content,tried both script and module".into(),
                            msg: "syntax error".into(),
                            span: Some(span),
                            help: Some("Run `nu-check --help` for more details".into()),
                            inner: vec![],
                        })
                    } else {
                        Ok(PipelineData::Value(Value::bool(false, span), None))
                    }
                }
            }
        }
    }
}

fn heuristic_parse_file(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    is_debug: bool,
) -> Result<PipelineData, ShellError> {
    let starting_error_count = working_set.parse_errors.len();
    let bytes = working_set.get_span_contents(call.head);
    let (filename, err) = unescape_unquote_string(bytes, call.head);
    if let Some(err) = err {
        working_set.error(err);
    }
    if starting_error_count == working_set.parse_errors.len() {
        if let Ok(contents) = std::fs::read(path) {
            match parse_script(
                working_set,
                Some(filename.as_str()),
                &contents,
                is_debug,
                call.head,
            ) {
                Ok(v) => Ok(v),
                Err(_) => {
                    match parse_module(working_set, Some(filename), &contents, is_debug, call.head)
                    {
                        Ok(v) => Ok(v),
                        Err(_) => {
                            if is_debug {
                                Err(ShellError::GenericError {
                                    error: "Failed to parse content,tried both script and module"
                                        .into(),
                                    msg: "syntax error".into(),
                                    span: Some(call.head),
                                    help: Some("Run `nu-check --help` for more details".into()),
                                    inner: vec![],
                                })
                            } else {
                                Ok(PipelineData::Value(Value::bool(false, call.head), None))
                            }
                        }
                    }
                }
            }
        } else {
            Err(ShellError::IOError {
                msg: "Can not read input".to_string(),
            })
        }
    } else {
        Err(ShellError::NotFound { span: call.head })
    }
}

fn parse_module(
    working_set: &mut StateWorkingSet,
    filename: Option<String>,
    contents: &[u8],
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let filename = filename.unwrap_or_else(|| "empty".to_string());

    let file_id = working_set.add_file(filename.clone(), contents);
    let new_span = working_set.get_span_for_file(file_id);

    let starting_error_count = working_set.parse_errors.len();
    parse_module_block(working_set, new_span, filename.as_bytes());

    if starting_error_count != working_set.parse_errors.len() {
        if is_debug {
            let msg = format!(
                r#"Found : {}"#,
                working_set
                    .parse_errors
                    .first()
                    .expect("Unable to parse content as module")
            );
            Err(ShellError::GenericError {
                error: "Failed to parse content".into(),
                msg,
                span: Some(span),
                help: Some("If the content is intended to be a script, please try to remove `--as-module` flag ".into()),
                inner: vec![],
            })
        } else {
            Ok(PipelineData::Value(Value::bool(false, new_span), None))
        }
    } else {
        Ok(PipelineData::Value(Value::bool(true, new_span), None))
    }
}

fn parse_script(
    working_set: &mut StateWorkingSet,
    filename: Option<&str>,
    contents: &[u8],
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let starting_error_count = working_set.parse_errors.len();
    parse(working_set, filename, contents, false);
    if starting_error_count != working_set.parse_errors.len() {
        let msg = format!(
            r#"Found : {}"#,
            working_set
                .parse_errors
                .first()
                .expect("Unable to parse content")
        );
        if is_debug {
            Err(ShellError::GenericError {
                error: "Failed to parse content".into(),
                msg,
                span: Some(span),
                help: Some("If the content is intended to be a module, please consider flag of `--as-module` ".into()),
                inner: vec![],
            })
        } else {
            Ok(PipelineData::Value(Value::bool(false, span), None))
        }
    } else {
        Ok(PipelineData::Value(Value::bool(true, span), None))
    }
}

fn parse_file_script(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    is_debug: bool,
) -> Result<PipelineData, ShellError> {
    let starting_error_count = working_set.parse_errors.len();
    let bytes = working_set.get_span_contents(call.head);
    let (filename, err) = unescape_unquote_string(bytes, call.head);
    if let Some(err) = err {
        working_set.error(err)
    }
    if starting_error_count == working_set.parse_errors.len() {
        if let Ok(contents) = std::fs::read(path) {
            parse_script(
                working_set,
                Some(filename.as_str()),
                &contents,
                is_debug,
                call.head,
            )
        } else {
            Err(ShellError::IOError {
                msg: "Can not read path".to_string(),
            })
        }
    } else {
        Err(ShellError::NotFound { span: call.head })
    }
}

fn parse_file_module(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    is_debug: bool,
) -> Result<PipelineData, ShellError> {
    let starting_error_count = working_set.parse_errors.len();
    let bytes = working_set.get_span_contents(call.head);
    let (filename, err) = unescape_unquote_string(bytes, call.head);
    if let Some(err) = err {
        working_set.error(err);
    }
    if starting_error_count == working_set.parse_errors.len() {
        if let Ok(contents) = std::fs::read(path) {
            parse_module(working_set, Some(filename), &contents, is_debug, call.head)
        } else {
            Err(ShellError::IOError {
                msg: "Can not read path".to_string(),
            })
        }
    } else {
        Err(ShellError::NotFound { span: call.head })
    }
}
