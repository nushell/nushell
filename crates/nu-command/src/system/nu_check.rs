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
        "nu check"
    }

    fn signature(&self) -> Signature {
        Signature::build("nu check")
            .required("path", SyntaxShape::Filepath, "the file path to check")
            .switch("as-module", "Parse content as module", Some('m'))
            .category(Category::Strings)
    }

    fn usage(&self) -> &str {
        "Check if input could be parsed correctly or not"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["syntax", "parse"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let path: Option<Spanned<String>> = call.opt(engine_state, stack, 0)?;
        let is_module = call.has_flag("as-module");

        // DO NOT ever try to merge the working_set in this command
        let mut working_set = StateWorkingSet::new(engine_state);
        let (path, span) = match find_path(path, engine_state, stack, call.head) {
            Ok((path, span)) => (path, span),
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
            parse_module(path, &mut working_set, call, span)
        } else {
            parse_script(path, &mut working_set, call, span)
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Parse input as script",
                example: "nu check script.nu",
                result: None,
            },
            Example {
                description: "Parse input as module",
                example: "nu check --as-module module.nu",
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
) -> Result<(String, Span), ShellError> {
    let cwd = current_dir(engine_state, stack)?;

    let (path, span) = match path {
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
            (path.to_string_lossy().to_string(), s.span)
        }
        None => {
            return Err(ShellError::NotFound(span));
        }
    };
    Ok((path, span))
}

fn parse_script(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let (filename, err) = unescape_unquote_string(path.as_bytes(), span);
    if err.is_none() {
        if let Ok(contents) = std::fs::read(&path) {
            let (_, err) = parse(working_set, Some(&filename), &contents, false, &[]);

            if err.is_some() {
                let msg = format!(r#"Found : {}"#, err.expect("Unable to parse content"));
                Err(ShellError::GenericError(
                    "Failed to parse content".to_string(),
                    msg,
                    Some(call.head),
                    Some("If the content is a module, please use --as-module flag".to_string()),
                    Vec::new(),
                ))
            } else {
                Ok(PipelineData::Value(
                    Value::string("Parse Success!", span),
                    None,
                ))
            }
        } else {
            Err(ShellError::IOError("Can not read path".to_string()))
        }
    } else {
        Err(ShellError::NotFound(span))
    }
}

fn parse_module(
    path: String,
    working_set: &mut StateWorkingSet,
    call: &Call,
    span: Span,
) -> Result<PipelineData, ShellError> {
    let (filename, err) = unescape_unquote_string(path.as_bytes(), span);
    if err.is_none() {
        if let Ok(contents) = std::fs::read(path) {
            let start = working_set.next_span_start();
            working_set.add_file(filename, &contents);
            let end = working_set.next_span_start();

            let new_span = Span::new(start, end);

            let (_, _, err) = parse_module_block(working_set, new_span, &[]);

            if err.is_some() {
                let msg = format!(
                    r#"Found : {}"#,
                    err.expect("Unable to parse content as module")
                );
                Err(ShellError::GenericError(
                    "Failed to parse module".to_string(),
                    msg,
                    Some(call.head),
                    Some("If the content is a script, please remove flag".to_string()),
                    Vec::new(),
                ))
            } else {
                Ok(PipelineData::Value(
                    Value::string("Parse Success!", span),
                    None,
                ))
            }
        } else {
            Err(ShellError::IOError("Can not read path".to_string()))
        }
    } else {
        Err(ShellError::NotFound(span))
    }
}
