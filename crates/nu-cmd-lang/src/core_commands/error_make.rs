use nu_engine::command_prelude::*;
use nu_protocol::{FromValue, LabeledError};

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

    fn extra_description(&self) -> &str {
        r#"Errors are defined by an `error_record`, which is a record with a specific
structure. (`*`) indicates a required key:

  * `msg: string` (`*`)
  * `code: string`
  * `label: oneof<table, record>`
  * `labels: table`
  * `help: string`
  * `url: string`
  * `inner: table`

The `label` and `labels` keys allow for placing arrows to points in the code,
optionally using `span` to find it (see `metadata`). `labels` must be a table,
while `label` can either be be a single record or a table. They have the
following record structure:

  * `text: string` (`*`)
  * `span: record<start: int end: int>`

The `inner` table takes a list of `error_struct` records, and can be used to
have detail the errors that happened in a previous `try {} catch {}` statement
or can be manually created. To use them from a `catch` statement, see the
example below."#
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
        let value: Value = call.req(engine_state, stack, 0)?;

        let throw_span = if call.has_flag(engine_state, stack, "unspanned")? {
            None
        } else {
            Some(call.head)
        };

        Err(make_other_error(&value, throw_span))
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
            label: {  # optional, can be a list of these records as well for multiple labels.
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
            Example {
                description: "Create a nested error from a try/catch statement with multiple labels",
                example: r#"try {
        error make {msg: "foo" labels: [{text: one} {text: two}]}
    } catch {|err|
        error make {msg: "bar", inner: [($err.json | from json)]}
    }"#,
                result: None,
            },
        ]
    }
}

// Most of the parsing happens with FromValue
#[derive(Debug, Default, Clone, FromValue)]
struct Error {
    msg: String,
    labels: Option<Vec<Label>>,
    label: Option<Value>,
    inner: Option<Vec<Value>>,
    help: Option<String>,
    url: Option<String>,
    code: Option<String>,
}

// Labels are parse separately because they could be vectors or single values.
#[derive(Debug, Default, Clone, FromValue)]
struct Label {
    text: String,
    span: Option<Span>,
}

fn make_other_error(value: &Value, throw_span: Option<Span>) -> ShellError {
    match Error::from_value(value.clone()) {
        Err(e) => e,
        Ok(v) => {
            // Main error that will be returned
            let mut error = LabeledError::new(v.msg);
            // Vec<Result<Label, ShellError>>
            let mut labels = Vec::new();
            if let Some(lab) = v.label {
                if let Ok(multi) = lab.as_list() {
                    for l in multi {
                        labels.push(Label::from_value(l.clone()));
                    }
                } else {
                    labels.push(Label::from_value(lab.clone()))
                }
            } else {
                labels.push(Ok(Label {
                    text: "originates from here".into(),
                    span: throw_span,
                }))
            }
            labels.extend(v.labels.unwrap_or_default().iter().map(|l| Ok(l.clone())));
            if let Some(ts) = throw_span {
                for label in labels {
                    match label {
                        Ok(lab) => error = error.with_label(lab.text, lab.span.unwrap_or(ts)),
                        Err(e) => return e,
                    }
                }
            }
            // Recurse into the inner errors
            for inner in v.inner.unwrap_or_default() {
                error = error.with_inner(make_other_error(&inner, Some(inner.span())));
            }
            error.code = v.code;
            error.help = v.help;
            error.url = v.url;
            error.into()
        }
    }
}
