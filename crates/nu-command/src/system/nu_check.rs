use nu_engine::{current_dir, CallExt};
use nu_parser::{parse, parse_module_block, unescape_unquote_string};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct NuCheck;

impl Command for NuCheck {
    fn name(&self) -> &str {
        "nu-check"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu-check")
            .optional("path", SyntaxShape::Filepath, "File path to parse")
            .switch("as-module", "Parse content as module", Some('m'))
            .switch("debug", "Show error messages", Some('d'))
            .switch("all", "Parse content as script first, returns result if success, otherwise, try with module", Some('a'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Validate and parse input content"
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
        let is_module = call.has_flag("as-module");
        let is_debug = call.has_flag("debug");
        let is_all = call.has_flag("all");
        let config = engine_state.get_config();
        let mut contents = vec![];

        // DO NOT ever try to merge the working_set in this command
        let mut working_set = StateWorkingSet::new(engine_state);

        if is_all && is_module {
            return Err(ShellError::GenericError(
                "Detected command flags conflict".to_string(),
                "You cannot have both `--all` and `--as-module` on the same command line, please refer to `nu-check --help` for more details".to_string(),
                Some(call.head),
                None, vec![]));
        }

        match input {
            PipelineData::Value(Value::String { val, span }, ..) => {
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
                let raw_stream: Vec<_> = stream.stream.into_iter().collect();
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
                if path.is_some() {
                    let path = match find_path(path, engine_state, stack, call.head) {
                        Ok(path) => path,
                        Err(error) => return Err(error),
                    };

                    let ext: Vec<_> = path.rsplitn(2, '.').collect();
                    if ext[0] != "nu" {
                        return Err(ShellError::GenericError(
                            "Cannot parse input".to_string(),
                            "File extension must be the type of .nu".to_string(),
                            Some(call.head),
                            None,
                            Vec::new(),
                        ));
                    }

                    if is_all {
                        heuristic_parse_file(path, &mut working_set, call, is_debug)
                    } else if is_module {
                        parse_file_module(path, &mut working_set, call, is_debug)
                    } else {
                        parse_file_script(path, &mut working_set, call, is_debug)
                    }
                } else {
                    Err(ShellError::GenericError(
                        "Failed to execute command".to_string(),
                        "Please run 'nu-check --help' for more details".to_string(),
                        Some(call.head),
                        None,
                        Vec::new(),
                    ))
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
                example: "nu-check -d script.nu",
                result: None,
            },
            Example {
                description: "Parse an external stream as script by showing error message",
                example: "open foo.nu | nu-check -d script.nu",
                result: None,
            },
            Example {
                description: "Parse an internal stream as module by showing error message",
                example: "open module.nu | lines | nu-check -d --as-module module.nu",
                result: None,
            },
            Example {
                description: "Parse a string as script",
                example: "echo $'two(char nl)lines' | nu-check ",
                result: None,
            },
            Example {
                description: "Heuristically parse which begins with script first, if it sees a failure, try module afterwards",
                example: "nu-check -a script.nu",
                result: None,
            },
            Example {
                description: "Heuristically parse by showing error message",
                example: "open foo.nu | lines | nu-check -ad",
                result: None,
            },
        ]
    }
}

fn find_path(
    path: Option<Spanned<String>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    span: Span,
) -> Result<String, ShellError> {
    let cwd = current_dir(engine_state, stack)?;

    let path = match path {
        Some(s) => {
            let path_no_whitespace = &s.item.trim_end_matches(|x| matches!(x, '\x09'..='\x0d'));

            let path = match nu_path::canonicalize_with(path_no_whitespace, &cwd) {
                Ok(p) => {
                    if !p.is_file() {
                        return Err(ShellError::GenericError(
                            "Cannot parse input".to_string(),
                            "Path is not a file".to_string(),
                            Some(s.span),
                            None,
                            Vec::new(),
                        ));
                    } else {
                        p
                    }
                }

                Err(_) => {
                    return Err(ShellError::FileNotFound(s.span));
                }
            };
            path.to_string_lossy().to_string()
        }
        None => {
            return Err(ShellError::NotFound(span));
        }
    };
    Ok(path)
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
                        Err(ShellError::GenericError(
                            "Failed to parse content,tried both script and module".to_string(),
                            "syntax error".to_string(),
                            Some(span),
                            Some("Run `nu-check --help` for more details".to_string()),
                            Vec::new(),
                        ))
                    } else {
                        Ok(PipelineData::Value(Value::boolean(false, span), None))
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
    let (filename, err) = unescape_unquote_string(path.as_bytes(), call.head);
    if err.is_none() {
        if let Ok(contents) = std::fs::read(&path) {
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
                                Err(ShellError::GenericError(
                                    "Failed to parse content,tried both script and module"
                                        .to_string(),
                                    "syntax error".to_string(),
                                    Some(call.head),
                                    Some("Run `nu-check --help` for more details".to_string()),
                                    Vec::new(),
                                ))
                            } else {
                                Ok(PipelineData::Value(Value::boolean(false, call.head), None))
                            }
                        }
                    }
                }
            }
        } else {
            Err(ShellError::IOError("Can not read input".to_string()))
        }
    } else {
        Err(ShellError::NotFound(call.head))
    }
}

fn parse_module(
    working_set: &mut StateWorkingSet,
    filename: Option<String>,
    contents: &[u8],
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let start = working_set.next_span_start();
    working_set.add_file(filename.unwrap_or_else(|| "empty".to_string()), contents);
    let end = working_set.next_span_start();

    let new_span = Span::new(start, end);
    let (_, _, err) = parse_module_block(working_set, new_span, &[]);

    if err.is_some() {
        if is_debug {
            let msg = format!(
                r#"Found : {}"#,
                err.expect("Unable to parse content as module")
            );
            Err(ShellError::GenericError(
                "Failed to parse content".to_string(),
                msg,
                Some(span),
                Some("If the content is intended to be a script, please try to remove `--as-module` flag ".to_string()),
                Vec::new(),
            ))
        } else {
            Ok(PipelineData::Value(Value::boolean(false, new_span), None))
        }
    } else {
        Ok(PipelineData::Value(Value::boolean(true, new_span), None))
    }
}

fn parse_script(
    working_set: &mut StateWorkingSet,
    filename: Option<&str>,
    contents: &[u8],
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let (_, err) = parse(working_set, filename, contents, false, &[]);
    if err.is_some() {
        let msg = format!(r#"Found : {}"#, err.expect("Unable to parse content"));
        if is_debug {
            Err(ShellError::GenericError(
                "Failed to parse content".to_string(),
                msg,
                Some(span),
                Some("If the content is intended to be a module, please consider flag of `--as-module` ".to_string()),
                Vec::new(),
            ))
        } else {
            Ok(PipelineData::Value(Value::boolean(false, span), None))
        }
    } else {
        Ok(PipelineData::Value(Value::boolean(true, span), None))
    }
}

fn parse_file_script(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    is_debug: bool,
) -> Result<PipelineData, ShellError> {
    let (filename, err) = unescape_unquote_string(path.as_bytes(), call.head);
    if err.is_none() {
        if let Ok(contents) = std::fs::read(&path) {
            parse_script(
                working_set,
                Some(filename.as_str()),
                &contents,
                is_debug,
                call.head,
            )
        } else {
            Err(ShellError::IOError("Can not read path".to_string()))
        }
    } else {
        Err(ShellError::NotFound(call.head))
    }
}

fn parse_file_module(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    is_debug: bool,
) -> Result<PipelineData, ShellError> {
    let (filename, err) = unescape_unquote_string(path.as_bytes(), call.head);
    if err.is_none() {
        if let Ok(contents) = std::fs::read(path) {
            parse_module(working_set, Some(filename), &contents, is_debug, call.head)
        } else {
            Err(ShellError::IOError("Can not read path".to_string()))
        }
    } else {
        Err(ShellError::NotFound(call.head))
    }
}
