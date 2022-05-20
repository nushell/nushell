use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct ErrorMake;

impl Command for ErrorMake {
    fn name(&self) -> &str {
        "error make"
    }

    fn signature(&self) -> Signature {
        Signature::build("error make")
            .optional("error_struct", SyntaxShape::Record, "the error to create")
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Create an error."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let span = call.head;
        let ctrlc = engine_state.ctrlc.clone();
        let arg: Option<Value> = call.opt(engine_state, stack, 0)?;

        if let Some(arg) = arg {
            Ok(make_error(&arg, span)
                .map(|err| Value::Error { error: err })
                .unwrap_or_else(|| Value::Error {
                    error: ShellError::GenericError(
                        "Creating error value not supported.".into(),
                        "unsupported error format".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    ),
                })
                .into_pipeline_data())
        } else {
            input.map(
                move |value| {
                    make_error(&value, span)
                        .map(|err| Value::Error { error: err })
                        .unwrap_or_else(|| Value::Error {
                            error: ShellError::GenericError(
                                "Creating error value not supported.".into(),
                                "unsupported error format".into(),
                                Some(span),
                                None,
                                Vec::new(),
                            ),
                        })
                },
                ctrlc,
            )
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

fn make_error(value: &Value, throw_span: Span) -> Option<ShellError> {
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
                        Some(throw_span),
                        None,
                        Vec::new(),
                    )),
                    _ => None,
                }
            }
            (Some(Value::String { val: message, .. }), None) => Some(ShellError::GenericError(
                message,
                "originates from here".to_string(),
                Some(throw_span),
                None,
                Vec::new(),
            )),
            _ => None,
        }
    } else {
        None
    }
}
