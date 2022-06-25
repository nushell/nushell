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
        let config = engine_state.get_config();
        let mut contents = vec![];

        // DO NOT ever try to merge the working_set in this command
        let mut working_set = StateWorkingSet::new(engine_state);

        match input {
            PipelineData::Value(Value::String { val, span }, ..) => {
                let contents = Vec::from(val);
                if is_module {
                    parse_module(&mut working_set, None, contents, is_debug, span)
                } else {
                    parse_script(&mut working_set, contents, None, is_debug, span)
                }
            }
            PipelineData::ListStream(stream, ..) => {
                let list_stream = stream.into_string(" ", config);
                let contents = Vec::from(list_stream);

                if is_module {
                    parse_module(&mut working_set, None, contents, is_debug, call.head)
                } else {
                    parse_script(&mut working_set, contents, None, is_debug, call.head)
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

                if is_module {
                    parse_module(&mut working_set, None, contents, is_debug, call.head)
                } else {
                    parse_script(&mut working_set, contents, None, is_debug, call.head)
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
                            "File extension must be .nu".to_string(),
                            Some(call.head),
                            None,
                            Vec::new(),
                        ));
                    }

                    if is_module {
                        parse_file_module(path, &mut working_set, call, is_debug)
                    } else {
                        parse_file_script(path, &mut working_set, call, is_debug)
                    }
                } else {
                    Err(ShellError::GenericError(
                        "Failed to execute command".to_string(),
                        "Do not understand the input, please run 'nu-check --help' for more details".to_string(),
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
                description: "Parse a file as script(Default)",
                example: "nu-check script.nu",
                result: None,
            },
            Example {
                description: "Parse a file input as module",
                example: "nu-check --as-module module.nu",
                result: None,
            },
            Example {
                description: "Parse a file with error message",
                example: "nu-check -d script.nu",
                result: None,
            },
            Example {
                description: "Parse an external stream as script with more error message",
                example: "open foo.nu | nu-check -d script.nu",
                result: None,
            },
            Example {
                description: "Parse an internal stream as module with more error message",
                example: "open module.nu | lines | nu-check -d --as-module module.nu",
                result: None,
            },
            Example {
                description: "Parse a string as script",
                example: "echo $'two(char nl)lines' | nu-check ",
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

fn parse_module(
    working_set: &mut StateWorkingSet,
    filename: Option<String>,
    contents: Vec<u8>,
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let start = working_set.next_span_start();
    working_set.add_file(
        filename.unwrap_or_else(|| "empty".to_string()),
        contents.as_ref(),
    );
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
                "Failed to parse module".to_string(),
                msg,
                Some(span),
                Some("If the content is a script, please remove flag".to_string()),
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
    contents: Vec<u8>,
    filename: Option<&str>,
    is_debug: bool,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let (_, err) = parse(working_set, filename, &contents, false, &[]);
    if err.is_some() {
        let msg = format!(r#"Found : {}"#, err.expect("Unable to parse content"));
        if is_debug {
            Err(ShellError::GenericError(
                "Failed to parse content".to_string(),
                msg,
                Some(span),
                Some("If the content is a module, please use --as-module flag".to_string()),
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
                contents,
                Some(filename.as_str()),
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
            parse_module(working_set, Some(filename), contents, is_debug, call.head)
        } else {
            Err(ShellError::IOError("Can not read path".to_string()))
        }
    } else {
        Err(ShellError::NotFound(call.head))
    }
}
