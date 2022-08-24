use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct ErrorMake;

impl Command for ErrorMake {
    fn name(&self) -> &str {
        "error make"
    }

    fn signature(&self) -> Signature {
        Signature::build("error make")
            .required("error_struct", SyntaxShape::Record, "the error to create")
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;
        let arg: Value = call.req(engine_state, stack, 0)?;
        let unspanned = call.has_flag("unspanned");

        if unspanned {
            Err(make_error(&arg, None).unwrap_or_else(|| {
                ShellError::GenericError(
                    "Creating error value not supported.".into(),
                    "unsupported error format".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )
            }))
        } else {
            Err(make_error(&arg, Some(span)).unwrap_or_else(|| {
                ShellError::GenericError(
                    "Creating error value not supported.".into(),
                    "unsupported error format".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )
            }))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a custom error for a custom command",
                example: r#"def foo [x] {
      let span = (metadata $x).span;
      error make {msg: "this is fishy", label: {text: "fish right here", start: $span.start, end: $span.end } }
    }"#,
                result: None,
            },
            Example {
                description: "Create a simple custom error for a custom command",
                example: r#"def foo [x] {
      error make {msg: "this is fishy"}
    }"#,
                result: None,
            },
        ]
    }
}

fn make_error(value: &Value, throw_span: Option<Span>) -> Option<ShellError> {
    if let Value::Record { .. } = &value {
        let msg = value.get_data_by_key("msg");
        let label = value.get_data_by_key("label");

        match (msg, &label) {
            (Some(Value::String { val: message, .. }), Some(label)) => {
                let label_start = label.get_data_by_key("start");
                let label_end = label.get_data_by_key("end");
                let label_text = label.get_data_by_key("text");

                match (label_start, label_end, label_text) {
                    (
                        Some(Value::Int { val: start, .. }),
                        Some(Value::Int { val: end, .. }),
                        Some(Value::String {
                            val: label_text, ..
                        }),
                    ) => Some(ShellError::GenericError(
                        message,
                        label_text,
                        Some(Span {
                            start: start as usize,
                            end: end as usize,
                        }),
                        None,
                        Vec::new(),
                    )),
                    (
                        None,
                        None,
                        Some(Value::String {
                            val: label_text, ..
                        }),
                    ) => Some(ShellError::GenericError(
                        message,
                        label_text,
                        throw_span,
                        None,
                        Vec::new(),
                    )),
                    _ => None,
                }
            }
            (Some(Value::String { val: message, .. }), None) => Some(ShellError::GenericError(
                message,
                "originates from here".to_string(),
                throw_span,
                None,
                Vec::new(),
            )),
            _ => None,
        }
    } else {
        None
    }
}
