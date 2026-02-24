use nu_engine::command_prelude::*;
use nu_protocol::{ErrorLabel, ErrorSource, FromValue, IntoValue, LabeledError};

#[derive(Clone)]
pub struct ErrorMake;

impl Command for ErrorMake {
    fn name(&self) -> &str {
        "error make"
    }

    fn signature(&self) -> Signature {
        Signature::build("error make")
            .category(Category::Core)
            .input_output_types(vec![(Type::Any, Type::Error)])
            .optional(
                "error_struct",
                SyntaxShape::OneOf(vec![SyntaxShape::Record(vec![]), SyntaxShape::String]),
                "The error to create.",
            )
            .switch("unspanned", "Remove the labels from the error.", Some('u'))
    }

    fn description(&self) -> &str {
        "Create an error."
    }

    fn extra_description(&self) -> &str {
        "Use either as a command with an `error_struct` or string as an input. The
`error_struct` is detailed below:

  * `msg: string` (required) 
  * `code: string`
  * `labels: table<error_label>`
  * `help: string`
  * `url: string`
  * `inner: table<error_struct>`
  * `src: src_record`

The `error_label` should contain the following keys:

  * `text: string`
  * `span: record<start: int end: int>`

External errors (referencing external sources, not the default nu spans) are
created using the `src` column with the `src_record` record. This only changes
where the labels are placed. For this, the `code` key is ignored, and will
always be `nu::shell::outside`. Errors cannot use labels that reference both
inside and outside sources, to do that use an `inner` error.

  * `name: string` - name of the source
  * `text: string` - the raw text to place the spans in
  * `path: string` - a file path to place the spans in

Errors can be chained together using the `inner` key, and multiple spans can be
specified to give more detailed error outputs.

If a string is passed it will be the `msg` part of the `error_struct`.

Errors can also be chained using `try {} catch {}`, allowing for related errors
to be printed out more easily. The code block for `catch` passes a record of the
`try` block's error into the catch block, which can be used in `error make`
either as the input or as an argument. These will be added as `inner` errors to
the most recent `error make`."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a simple, default error.",
                example: "error make",
                result: None,
            },
            Example {
                description: "Create a simple error from a string.",
                example: "error make 'my error message'",
                result: None,
            },
            Example {
                description: "Create a simple error from an `error_struct` record.",
                example: "error make {msg: 'my error message'}",
                result: None,
            },
            Example {
                description: "A complex error utilizing spans and inners.",
                example: r#"def foo [x: int, y: int] {
        let z = $x + $y
        error make {
            msg: "an error for foo just occurred"
            labels: [
                {text: "one" span: (metadata $x).span}
                {text: "two" span: (metadata $y).span}
            ]
            help: "some help for the user"
            inner: [
                {msg: "an inner error" labels: [{text: "" span: (metadata $y).span}]}
            ]
        }
    }"#,
                result: None,
            },
            Example {
                description: "Chain errors using a pipeline.",
                example: r#"try {error make "foo"} catch {error make "bar"}"#,
                result: None,
            },
            Example {
                description: "Chain errors using arguments (note the extra command in `catch`).",
                example: r#"try {
        error make "foo"
    } catch {|err|
        print 'We got an error that will be chained!'
        error make {msg: "bar" inner: [$err]}
    }"#,
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value = match call.opt(engine_state, stack, 0) {
            Ok(Some(v @ Value::Record { .. } | v @ Value::String { .. })) => v,
            Ok(_) => Value::string("originates from here", call.head),
            Err(e) => return Err(e),
        };
        let show_labels: bool = !call.has_flag(engine_state, stack, "unspanned")?;

        let inners = match ErrorInfo::from_value(input.into_value(call.head)?) {
            Ok(v) => vec![v.into_value(call.head)],
            Err(_) => vec![],
        };

        Err(match (inners, value) {
            (inner, Value::String { val, .. }) => ErrorInfo {
                msg: val,
                inner,
                ..ErrorInfo::default()
            }
            .labeled(call.head, show_labels),
            (
                inner,
                Value::Record {
                    val, internal_span, ..
                },
            ) => {
                let mut ei = ErrorInfo::from_value((*val).clone().into_value(internal_span))?;
                ei.inner = [ei.inner, inner].concat();

                ei.labeled(internal_span, show_labels)
            }
            (_, Value::Error { error, .. }) => *error,
            _ => todo!(),
        })
    }
}

#[derive(Debug, Clone, IntoValue, FromValue)]
struct ErrorInfo {
    msg: String,
    code: Option<String>,
    help: Option<String>,
    url: Option<String>,
    #[nu_value(default)]
    labels: Vec<ErrorLabel>,
    label: Option<ErrorLabel>,
    #[nu_value(default)]
    inner: Vec<Value>,
    raw: Option<Value>,
    src: Option<ErrorSource>,
}

impl Default for ErrorInfo {
    fn default() -> Self {
        Self {
            msg: "Originates from here".into(),
            code: Some("nu::shell::error".into()),
            help: None,
            url: None,
            labels: Vec::default(),
            label: None,
            inner: Vec::default(),
            raw: None,
            src: None,
        }
    }
}

impl ErrorInfo {
    pub fn labels(self) -> Vec<ErrorLabel> {
        match self.label {
            None => self.labels,
            Some(label) => [self.labels, vec![label]].concat(),
        }
    }
    pub fn labeled(self, span: Span, show_labels: bool) -> ShellError {
        let inner: Vec<ShellError> = self
            .inner
            .clone()
            .into_iter()
            .map(|i| match ErrorInfo::from_value(i) {
                Ok(e) => e.labeled(span, show_labels),
                Err(err) => err,
            })
            .collect();
        let labels = self.clone().labels();

        match self {
            // External error with src code and url
            ErrorInfo {
                src: Some(src),
                url: Some(url),
                msg,
                help,
                raw: None,
                ..
            } => ShellError::OutsideSource {
                src: src.into(),
                labels: labels.into_iter().map(|l| l.into()).collect(),
                msg,
                url,
                help,
                inner,
            },
            // External error with src code
            ErrorInfo {
                src: Some(src),
                msg,
                help,
                raw: None,
                ..
            } => ShellError::OutsideSourceNoUrl {
                src: src.into(),
                labels: labels.into_iter().map(|l| l.into()).collect(),
                msg,
                help,
                inner,
            },
            // Normal error
            ei @ ErrorInfo {
                src: None,
                raw: None,
                ..
            } => LabeledError {
                labels: match (show_labels, labels.as_slice()) {
                    (true, []) => vec![ErrorLabel {
                        text: "".into(),
                        span,
                    }],
                    (true, labels) => labels.to_vec(),
                    (false, _) => vec![],
                }
                .into(),
                msg: ei.msg,
                code: ei.code,
                url: ei.url,
                help: ei.help,
                inner: inner.into(),
            }
            .into(),
            // Error error with a raw error value somewhere
            ErrorInfo { raw: Some(v), .. } => ShellError::from_value(v).unwrap_or_else(|e| e),
        }
    }
}
