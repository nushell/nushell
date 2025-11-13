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

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a simple custom error",
                example: r#"error make {msg: "my custom error message"}"#,
                result: None,
            },
            Example {
                description: "Create a complex error for a custom command that shows a full `error_struct`",
                example: r#"def foo [x] {
        error make {
            msg: "this is fishy"
            code: "my::error"  # optional error type to use
            label: {  # optional
                text: "fish right here"  # Required if $.label exists
                # use (metadata $var).span to get the {start: x end: y} of the variable
                span: (metadata $x).span  # optional
            }
            help: "something to tell the user as help"  # optional
            url: "https://nushell.sh"  # optional
        }
    }"#,
                result: None,
            },
        ]
    }
}

const UNABLE_TO_PARSE: &str = "Unable to parse error format.";

fn make_other_error(value: &Value, throw_span: Option<Span>) -> ShellError {
    let value_span = value.span();
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
                span: Some(value_span),
                help: None,
                inner: vec![],
            };
        }
        None => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "missing required member `$.msg`".into(),
                span: Some(value_span),
                help: None,
                inner: vec![],
            };
        }
    };

    let help = match value.get("help") {
        Some(Value::String { val, .. }) => Some(val.clone()),
        _ => None,
    };

    let (label, label_span): (&Record, Option<Span>) = match value.get("label") {
        Some(value @ Value::Record { val, .. }) => (val, Some(value.span())),
        Some(_) => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "`$.label` has wrong type, must be a record".into(),
                span: Some(value_span),
                help: None,
                inner: vec![],
            };
        }
        _ => (
            &record! {
                "text" => "originates from here".into_value(value_span),
            },
            throw_span,
        ),
    };

    // remove after a few versions
    if label.get("start").is_some() || label.get("end").is_some() {
        return ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: "`start` and `end` are deprecated".into(),
            span: Some(value_span),
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
                span: label_span,
                help: None,
                inner: vec![],
            };
        }
        None => {
            return ShellError::GenericError {
                error: UNABLE_TO_PARSE.into(),
                msg: "missing required member `$.label.text`".into(),
                span: label_span,
                help: None,
                inner: vec![],
            };
        }
    };

    let (this_span, span_span): (&Record, Option<Span>) = match label.get("span") {
        Some(value @ Value::Record { val, .. }) => (val, Some(value.span())),
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
        None => (&record!(), label_span),
    };

    let (span_start, span_end) = match get_span_sides(this_span, span_span, throw_span) {
        Ok((start, end)) => (start, end),
        Err(err) => return err,
    };

    if span_start > span_end {
        return ShellError::GenericError {
            error: "invalid error format.".into(),
            msg: "`$.label.start` should be smaller than `$.label.end`".into(),
            span: label_span,
            help: Some(format!("{span_start} > {span_end}")),
            inner: vec![],
        };
    }

    let code = match value.get("code") {
        Some(Value::String { val, .. }) => Some(val.clone()),
        _ => None,
    };

    let url = match value.get("url") {
        Some(Value::String { val, .. }) => Some(val.clone()),
        _ => None,
    };

    // correct return: everything present
    let mut error = LabeledError::new(msg);
    if span_end != -1 {
        error = error.with_label(text, Span::new(span_start as usize, span_end as usize));
    };
    error.code = code;
    error.help = help;
    error.url = url;
    error.into()
}

enum SpanResults {
    Ok(i64),
    NotInt(ShellError),
    MissingSide(ShellError),
}

fn get_span_side(span: &Record, span_span: Span, side: &str) -> SpanResults {
    match span.get(side) {
        Some(Value::Int { val, .. }) => SpanResults::Ok(*val),
        Some(_) => SpanResults::NotInt(ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: format!("`$.span.{side}` must be int"),
            span: Some(span_span),
            help: None,
            inner: vec![],
        }),
        None => SpanResults::MissingSide(ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: format!("`$.span.{side}` must be present, if span is specified."),
            span: Some(span_span),
            help: None,
            inner: vec![],
        }),
    }
}

fn get_span_sides(
    span: &Record,
    span_span: Option<Span>,
    cmd_span: Option<Span>,
) -> Result<(i64, i64), ShellError> {
    if span_span.is_none() || cmd_span.is_none() {
        return Ok((-1, -1));
    }
    let sides = (
        get_span_side(span, span_span.unwrap_or_default(), "start"),
        get_span_side(span, span_span.unwrap_or_default(), "end"),
    );

    match sides {
        // Both okay, return the span we were given
        (SpanResults::Ok(start), SpanResults::Ok(end)) => Ok((start, end)),
        // Missing both sides, so default to the `span_span`
        (SpanResults::MissingSide(_), SpanResults::MissingSide(_)) => Ok((
            cmd_span.unwrap_or_default().start as i64,
            cmd_span.unwrap_or_default().end as i64,
        )),
        // Missing one side, return an error
        (SpanResults::MissingSide(err), _) | (_, SpanResults::MissingSide(err)) => Err(err),
        // Otherwise:
        (SpanResults::Ok(_), SpanResults::NotInt(err))
        | (SpanResults::NotInt(err), SpanResults::Ok(_)) => Err(err),
        _ => Err(ShellError::GenericError {
            error: UNABLE_TO_PARSE.into(),
            msg: "`$.span` values must be ints".into(),
            span: span_span,
            help: None,
            inner: vec![],
        }),
    }
}
