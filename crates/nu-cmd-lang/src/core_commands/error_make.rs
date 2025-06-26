use nu_engine::command_prelude::*;
use nu_protocol::LabeledError;

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
                "The error to create.",
            )
            .switch(
                "unspanned",
                "remove the origin label from the error",
                Some('u'),
            )
            .category(Category::Core)
    }

    fn description(&self) -> &str {
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

        let throw_span = if call.has_flag(engine_state, stack, "unspanned")? {
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
                result: None,
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
                result: None,
            },
            Example {
                description: "Create a custom error for a custom command that shows the span of the argument",
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
    let span = value.span();
    let value = match value {
        Value::Record { val, .. } => val,
        _ => {
            return ShellError::GenericError {
                error: "Creating error value not supported.".into(),
                msg: "unsupported error format, must be a record".into(),
                span: throw_span,
                help: None,
                inner: vec![],
            };
        }
    };

    let msg = match value.get("msg") {
        Some(Value::String { val, .. }) => val.clone(),
        Some(_) => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "`$.msg` has wrong type, must be string".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            };
        }
        None => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "missing required member `$.msg`".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            };
        }
    };

    let help = match value.get("help") {
        Some(Value::String { val, .. }) => Some(val.clone()),
        _ => None,
    };

    let (label, label_span) = match value.get("label") {
        Some(value @ Value::Record { val, .. }) => (val, value.span()),
        Some(_) => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "`$.label` has wrong type, must be a record".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            };
        }
        // correct return: no label
        None => {
            return ShellError::GenericError {
                error: msg,
                msg: "originates from here".into(),
                span: throw_span,
                help,
                inner: vec![],
            };
        }
    };

    // remove after a few versions
    if label.get("start").is_some() || label.get("end").is_some() {
        return ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: "`start` and `end` are deprecated".into(),
            span: Some(span),
            help: Some("Use `$.label.span` instead".into()),
            inner: vec![],
        };
    }

    let text = match label.get("text") {
        Some(Value::String { val, .. }) => val.clone(),
        Some(_) => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "`$.label.text` has wrong type, must be string".into(),
                span: Some(label_span),
                help: None,
                inner: vec![],
            };
        }
        None => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "missing required member `$.label.text`".into(),
                span: Some(label_span),
                help: None,
                inner: vec![],
            };
        }
    };

    let (span, span_span) = match label.get("span") {
        Some(value @ Value::Record { val, .. }) => (val, value.span()),
        Some(value) => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "`$.label.span` has wrong type, must be record".into(),
                span: Some(value.span()),
                help: None,
                inner: vec![],
            };
        }
        // correct return: label, no span
        None => {
            return ShellError::GenericError {
                error: msg,
                msg: text,
                span: throw_span,
                help,
                inner: vec![],
            };
        }
    };

    let span_start = match get_span_sides(span, span_span, "start") {
        Ok(val) => val,
        Err(err) => return err,
    };
    let span_end = match get_span_sides(span, span_span, "end") {
        Ok(val) => val,
        Err(err) => return err,
    };

    if span_start > span_end {
        return ShellError::GenericError {
            error: "invalid error format.".into(),
            msg: "`$.label.start` should be smaller than `$.label.end`".into(),
            span: Some(label_span),
            help: Some(format!("{span_start} > {span_end}")),
            inner: vec![],
        };
    }

    // correct return: everything present
    let mut error =
        LabeledError::new(msg).with_label(text, Span::new(span_start as usize, span_end as usize));
    error.help = help;
    error.into()
}

fn get_span_sides(span: &Record, span_span: Span, side: &str) -> Result<i64, ShellError> {
    match span.get(side) {
        Some(Value::Int { val, .. }) => Ok(*val),
        Some(_) => Err(ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: format!("`$.span.{side}` must be int"),
            span: Some(span_span),
            help: None,
            inner: vec![],
        }),
        None => Err(ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: format!("`$.span.{side}` must be present, if span is specified."),
            span: Some(span_span),
            help: None,
            inner: vec![],
        }),
    }
}
