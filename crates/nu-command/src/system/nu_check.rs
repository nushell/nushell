use nu_engine::{command_prelude::*, find_in_dirs_env, get_dirs_var_from_call};
use nu_parser::{parse, parse_module_block, parse_module_file_or_dir, unescape_unquote_string};
use nu_protocol::{
    engine::{FileStack, StateWorkingSet},
    shell_error::io::IoError,
};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct NuCheck;

impl Command for NuCheck {
    fn name(&self) -> &str {
        "nu-check"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu-check")
            .input_output_types(vec![
                (Type::Nothing, Type::Bool),
                (Type::String, Type::Bool),
                (Type::List(Box::new(Type::Any)), Type::Bool),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::Bool),
            ])
            // type is string to avoid automatically canonicalizing the path
            .optional("path", SyntaxShape::String, "File path to parse.")
            .switch("as-module", "Parse content as module", Some('m'))
            .switch("debug", "Show error messages", Some('d'))
            .category(Category::Strings)
    }

    fn description(&self) -> &str {
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
        let path_arg: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        let as_module = call.has_flag(engine_state, stack, "as-module")?;
        let is_debug = call.has_flag(engine_state, stack, "debug")?;

        // DO NOT ever try to merge the working_set in this command
        let mut working_set = StateWorkingSet::new(engine_state);

        let input_span = input.span().unwrap_or(call.head);

        match input {
            PipelineData::Value(Value::String { val, .. }, ..) => {
                let contents = Vec::from(val);
                if as_module {
                    parse_module(&mut working_set, None, &contents, is_debug, input_span)
                } else {
                    parse_script(&mut working_set, None, &contents, is_debug, input_span)
                }
            }
            PipelineData::ListStream(stream, ..) => {
                let config = stack.get_config(engine_state);
                let list_stream = stream.into_string("\n", &config);
                let contents = Vec::from(list_stream);

                if as_module {
                    parse_module(&mut working_set, None, &contents, is_debug, call.head)
                } else {
                    parse_script(&mut working_set, None, &contents, is_debug, call.head)
                }
            }
            PipelineData::ByteStream(stream, ..) => {
                let contents = stream.into_bytes()?;

                if as_module {
                    parse_module(&mut working_set, None, &contents, is_debug, call.head)
                } else {
                    parse_script(&mut working_set, None, &contents, is_debug, call.head)
                }
            }
            _ => {
                if let Some(path_str) = path_arg {
                    let path_span = path_str.span;

                    // look up the path as relative to FILE_PWD or inside NU_LIB_DIRS (same process as source-env)
                    let path = match find_in_dirs_env(
                        &path_str.item,
                        engine_state,
                        stack,
                        get_dirs_var_from_call(stack, call),
                    ) {
                        Ok(Some(path)) => path,
                        Ok(None) => {
                            return Err(ShellError::Io(IoError::new(
                                ErrorKind::FileNotFound,
                                path_span,
                                PathBuf::from(path_str.item),
                            )));
                        }
                        Err(err) => return Err(err),
                    };

                    if as_module || path.is_dir() {
                        parse_file_or_dir_module(
                            path.to_string_lossy().as_bytes(),
                            &mut working_set,
                            is_debug,
                            path_span,
                            call.head,
                        )
                    } else {
                        // Unlike `parse_file_or_dir_module`, `parse_file_script` parses the content directly,
                        // without adding the file to the stack. Therefore we need to handle this manually.
                        working_set.files = FileStack::with_file(path.clone());
                        parse_file_script(&path, &mut working_set, is_debug, path_span, call.head)
                        // The working set is not merged, so no need to pop the file from the stack.
                    }
                } else {
                    Err(ShellError::GenericError {
                        error: "Failed to execute command".into(),
                        msg: "Requires path argument if ran without pipeline input".into(),
                        span: Some(call.head),
                        help: Some("Please run 'nu-check --help' for more details".into()),
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
                description: "Parse a byte stream as script by showing error message",
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
        ]
    }
}

fn parse_module(
    working_set: &mut StateWorkingSet,
    filename: Option<String>,
    contents: &[u8],
    is_debug: bool,
    call_head: Span,
) -> Result<PipelineData, ShellError> {
    let filename = filename.unwrap_or_else(|| "empty".to_string());

    let file_id = working_set.add_file(filename.clone(), contents);
    let new_span = working_set.get_span_for_file(file_id);

    let starting_error_count = working_set.parse_errors.len();
    parse_module_block(working_set, new_span, filename.as_bytes());

    check_parse(
        starting_error_count,
        working_set,
        is_debug,
        Some(
            "If the content is intended to be a script, please try to remove `--as-module` flag "
                .to_string(),
        ),
        call_head,
    )
}

fn parse_script(
    working_set: &mut StateWorkingSet,
    filename: Option<&str>,
    contents: &[u8],
    is_debug: bool,
    call_head: Span,
) -> Result<PipelineData, ShellError> {
    let starting_error_count = working_set.parse_errors.len();
    parse(working_set, filename, contents, false);
    check_parse(starting_error_count, working_set, is_debug, None, call_head)
}

fn check_parse(
    starting_error_count: usize,
    working_set: &StateWorkingSet,
    is_debug: bool,
    help: Option<String>,
    call_head: Span,
) -> Result<PipelineData, ShellError> {
    if starting_error_count != working_set.parse_errors.len() {
        let msg = format!(
            r#"Found : {}"#,
            working_set
                .parse_errors
                .first()
                .expect("Missing parser error")
        );

        if is_debug {
            Err(ShellError::GenericError {
                error: "Failed to parse content".into(),
                msg,
                span: Some(call_head),
                help,
                inner: vec![],
            })
        } else {
            Ok(PipelineData::Value(Value::bool(false, call_head), None))
        }
    } else {
        Ok(PipelineData::Value(Value::bool(true, call_head), None))
    }
}

fn parse_file_script(
    path: &Path,
    working_set: &mut StateWorkingSet,
    is_debug: bool,
    path_span: Span,
    call_head: Span,
) -> Result<PipelineData, ShellError> {
    let filename = check_path(working_set, path_span, call_head)?;

    match std::fs::read(path) {
        Ok(contents) => parse_script(working_set, Some(&filename), &contents, is_debug, call_head),
        Err(err) => Err(ShellError::Io(IoError::new(
            err.not_found_as(NotFound::File),
            path_span,
            PathBuf::from(path),
        ))),
    }
}

fn parse_file_or_dir_module(
    path_bytes: &[u8],
    working_set: &mut StateWorkingSet,
    is_debug: bool,
    path_span: Span,
    call_head: Span,
) -> Result<PipelineData, ShellError> {
    let _ = check_path(working_set, path_span, call_head)?;

    let starting_error_count = working_set.parse_errors.len();
    let _ = parse_module_file_or_dir(working_set, path_bytes, path_span, None);

    if starting_error_count != working_set.parse_errors.len() {
        if is_debug {
            let msg = format!(
                r#"Found : {}"#,
                working_set
                    .parse_errors
                    .first()
                    .expect("Missing parser error")
            );
            Err(ShellError::GenericError {
                error: "Failed to parse content".into(),
                msg,
                span: Some(path_span),
                help: Some("If the content is intended to be a script, please try to remove `--as-module` flag ".into()),
                inner: vec![],
            })
        } else {
            Ok(PipelineData::Value(Value::bool(false, call_head), None))
        }
    } else {
        Ok(PipelineData::Value(Value::bool(true, call_head), None))
    }
}

fn check_path(
    working_set: &mut StateWorkingSet,
    path_span: Span,
    call_head: Span,
) -> Result<String, ShellError> {
    let bytes = working_set.get_span_contents(path_span);
    let (filename, err) = unescape_unquote_string(bytes, path_span);
    if let Some(e) = err {
        Err(ShellError::GenericError {
            error: "Could not escape filename".to_string(),
            msg: "could not escape filename".to_string(),
            span: Some(call_head),
            help: Some(format!("Returned error: {e}")),
            inner: vec![],
        })
    } else {
        Ok(filename)
    }
}
