use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
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
        let arg: Value = call.req(engine_state, stack, 0)?;

        let throw_span = if call.has_flag("unspanned") {
            None
        } else {
            Some(call.head)
        };

        Err(make_other_error(&arg, throw_span))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a simple custom error",
                example: r#"error make {msg: "my custom error message"}"#,
                result: Some(Value::error(
                    ShellError::GenericError(
                        "my custom error message".to_string(),
                        "".to_string(),
                        None,
                        None,
                        Vec::new(),
                    ),
                    Span::unknown(),
                )),
            },
            Example {
                description: "Create a more complex custom error",
                example: r#"error make {
        msg: "my custom error message"
        label: {
            text: "my custom label text"  # not mandatory unless $.label exists
            # optional
            span: {
                # if $.label.span exists, both start and end must be present
                start: 123
                end: 456
            }
        }
        help: "A help string, suggesting a fix to the user"  # optional
    }"#,
                result: Some(Value::error(
                    ShellError::GenericError(
                        "my custom error message".to_string(),
                        "my custom label text".to_string(),
                        Some(Span::new(123, 456)),
                        Some("A help string, suggesting a fix to the user".to_string()),
                        Vec::new(),
                    ),
                    Span::unknown(),
                )),
            },
            Example {
                description:
                    "Create a custom error for a custom command that shows the span of the argument",
                example: r#"def foo [x] {
        error make {
            msg: "this is fishy"
            label: {
                text: "fish right here"
                span: (metadata $x).span
            }
        }
    }"#,
                result: None,
            },
        ]
    }
}

const UNABLE_TO_PARSE: &str = "Unable to parse error format.";

fn make_other_error(value: &Value, throw_span: Option<Span>) -> ShellError {
    let value = match value {
        Value::Record { .. } => value,
        _ => {
            return ShellError::GenericError(
                "Creating error value not supported.".into(),
                "unsupported error format, must be a record".into(),
                throw_span,
                None,
                Vec::new(),
            )
        }
    };

    let msg = match value.get_data_by_key("msg") {
        Some(Value::String { val, .. }) => val,
        Some(_) => {
            return ShellError::GenericError(
                UNABLE_TO_PARSE.into(),
                "`$.msg` has wrong type, must be string".into(),
                Some(value.span()),
                None,
                Vec::new(),
            )
        }
        None => {
            return ShellError::GenericError(
                UNABLE_TO_PARSE.into(),
                "missing required member `$.msg`".into(),
                Some(value.span()),
                None,
                Vec::new(),
            )
        }
    };

    let help = match value.get_data_by_key("help") {
        Some(Value::String { val, .. }) => Some(val),
        _ => None,
    };

    let label = match value.get_data_by_key("label") {
        Some(value) => value,
        // correct return: no label
        None => {
            return ShellError::GenericError(
                msg,
                "originates from here".to_string(),
                throw_span,
                help,
                Vec::new(),
            )
        }
    };

    // remove after a few versions
    if label.get_data_by_key("start").is_some() || label.get_data_by_key("end").is_some() {
        return ShellError::GenericError(
            UNABLE_TO_PARSE.into(),
            "`start` and `end` are deprecated".into(),
            Some(value.span()),
            Some("Use `$.label.span` instead".into()),
            Vec::new(),
        );
    }

    let text = match label.get_data_by_key("text") {
        Some(Value::String { val, .. }) => val,
        Some(_) => {
            return ShellError::GenericError(
                UNABLE_TO_PARSE.into(),
                "`$.label.text` has wrong type, must be string".into(),
                Some(label.span()),
                None,
                Vec::new(),
            )
        }
        None => {
            return ShellError::GenericError(
                UNABLE_TO_PARSE.into(),
                "missing required member `$.label.text`".into(),
                Some(label.span()),
                None,
                Vec::new(),
            )
        }
    };

    let span = match label.get_data_by_key("span") {
        Some(val @ Value::Record { .. }) => val,
        Some(value) => {
            return ShellError::GenericError(
                UNABLE_TO_PARSE.into(),
                "`$.label.span` has wrong type, must be record".into(),
                Some(value.span()),
                None,
                Vec::new(),
            )
        }
        // correct return: label, no span
        None => return ShellError::GenericError(msg, text, throw_span, help, Vec::new()),
    };

    let span_start = match get_span_sides(&span, "start") {
        Ok(val) => val,
        Err(err) => return err,
    };
    let span_end = match get_span_sides(&span, "end") {
        Ok(val) => val,
        Err(err) => return err,
    };

    if span_start > span_end {
        return ShellError::GenericError(
            "invalid error format.".into(),
            "`$.label.start` should be smaller than `$.label.end`".into(),
            Some(label.span()),
            Some(format!("{} > {}", span_start, span_end)),
            Vec::new(),
        );
    }

    // correct return: everything present
    ShellError::GenericError(
        msg,
        text,
        Some(Span::new(span_start as usize, span_end as usize)),
        help,
        Vec::new(),
    )
}

fn get_span_sides(span: &Value, side: &str) -> Result<i64, ShellError> {
    match span.get_data_by_key(side) {
        Some(Value::Int { val, .. }) => Ok(val),
        Some(_) => Err(ShellError::GenericError(
            UNABLE_TO_PARSE.into(),
            format!("`$.span.{side}` must be int"),
            Some(span.span()),
            None,
            Vec::new(),
        )),
        None => Err(ShellError::GenericError(
            UNABLE_TO_PARSE.into(),
            format!("`$.span.{side}` must be present, if span is specified."),
            Some(span.span()),
            None,
            Vec::new(),
        )),
    }
}
