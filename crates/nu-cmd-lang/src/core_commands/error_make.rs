use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SpannedValue, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct ErrorMake;

impl Command for ErrorMake {
    fn name(&self) -> &str {
        "error make"
    }

    fn signature(&self) -> Signature {
        Signature::build("error make")
            .input_output_types(vec![(Type::Nothing, Type::Error)])
            .required(
                "error_struct",
                SyntaxShape::Record(vec![]),
                "the error to create",
            )
            .switch(
                "unspanned",
                "remove the origin label from the error",
                Some('u'),
            )
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Create an error."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["panic", "crash", "throw"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let arg: SpannedValue = call.req(engine_state, stack, 0)?;
        let unspanned = call.has_flag("unspanned");

        let throw_error = if unspanned { None } else { Some(span) };
        Err(make_error(&arg, throw_error).unwrap_or_else(|| {
            ShellError::GenericError(
                "Creating error value not supported.".into(),
                "unsupported error format".into(),
                Some(span),
                None,
                Vec::new(),
            )
        }))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a simple custom error",
                example: r#"error make {msg: "my custom error message"}"#,
                result: Some(SpannedValue::Error {
                    error: Box::new(ShellError::GenericError(
                        "my custom error message".to_string(),
                        "".to_string(),
                        None,
                        None,
                        Vec::new(),
                    )),
                }),
            },
            Example {
                description: "Create a more complex custom error",
                example: r#"error make {
        msg: "my custom error message"
        label: {
            text: "my custom label text"  # not mandatory unless $.label exists
            start: 123  # not mandatory unless $.label.end is set
            end: 456  # not mandatory unless $.label.start is set
        }
    }"#,
                result: Some(SpannedValue::Error {
                    error: Box::new(ShellError::GenericError(
                        "my custom error message".to_string(),
                        "my custom label text".to_string(),
                        Some(Span::new(123, 456)),
                        None,
                        Vec::new(),
                    )),
                }),
            },
            Example {
                description:
                    "Create a custom error for a custom command that shows the span of the argument",
                example: r#"def foo [x] {
        let span = (metadata $x).span;
        error make {
            msg: "this is fishy"
            label: {
                text: "fish right here"
                start: $span.start
                end: $span.end
            }
        }
    }"#,
                result: None,
            },
        ]
    }
}

fn make_error(value: &SpannedValue, throw_span: Option<Span>) -> Option<ShellError> {
    if let SpannedValue::Record { span, .. } = &value {
        let msg = value.get_data_by_key("msg");
        let label = value.get_data_by_key("label");

        match (msg, &label) {
            (Some(SpannedValue::String { val: message, .. }), Some(label)) => {
                let label_start = label.get_data_by_key("start");
                let label_end = label.get_data_by_key("end");
                let label_text = label.get_data_by_key("text");

                let label_span = match label.span() {
                    Ok(lspan) => Some(lspan),
                    Err(_) => None,
                };

                match (label_start, label_end, label_text) {
                    (
                        Some(SpannedValue::Int { val: start, .. }),
                        Some(SpannedValue::Int { val: end, .. }),
                        Some(SpannedValue::String {
                            val: label_text, ..
                        }),
                    ) => {
                        if start > end {
                            Some(ShellError::GenericError(
                                "invalid error format.".into(),
                                "`$.label.start` should be smaller than `$.label.end`".into(),
                                label_span,
                                Some(format!("{} > {}", start, end)),
                                Vec::new(),
                            ))
                        } else {
                            Some(ShellError::GenericError(
                                message,
                                label_text,
                                Some(Span::new(start as usize, end as usize)),
                                None,
                                Vec::new(),
                            ))
                        }
                    }
                    (
                        None,
                        None,
                        Some(SpannedValue::String {
                            val: label_text, ..
                        }),
                    ) => Some(ShellError::GenericError(
                        message,
                        label_text,
                        throw_span,
                        None,
                        Vec::new(),
                    )),
                    (_, _, None) => Some(ShellError::GenericError(
                        "Unable to parse error format.".into(),
                        "missing required member `$.label.text`".into(),
                        label_span,
                        None,
                        Vec::new(),
                    )),
                    (Some(SpannedValue::Int { .. }), None, _) => Some(ShellError::GenericError(
                        "Unable to parse error format.".into(),
                        "missing required member `$.label.end`".into(),
                        label_span,
                        Some("required because `$.label.start` is set".to_string()),
                        Vec::new(),
                    )),
                    (None, Some(SpannedValue::Int { .. }), _) => Some(ShellError::GenericError(
                        "Unable to parse error format.".into(),
                        "missing required member `$.label.start`".into(),
                        label_span,
                        Some("required because `$.label.end` is set".to_string()),
                        Vec::new(),
                    )),
                    _ => None,
                }
            }
            (Some(SpannedValue::String { val: message, .. }), None) => {
                Some(ShellError::GenericError(
                    message,
                    "originates from here".to_string(),
                    throw_span,
                    None,
                    Vec::new(),
                ))
            }
            (None, _) => Some(ShellError::GenericError(
                "Unable to parse error format.".into(),
                "missing required member `$.msg`".into(),
                Some(*span),
                None,
                Vec::new(),
            )),
            _ => None,
        }
    } else {
        None
    }
}
