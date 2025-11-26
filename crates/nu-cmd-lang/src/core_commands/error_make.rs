use nu_engine::command_prelude::*;
use nu_protocol::{ErrorLabel, FromValue, IntoValue, LabeledError};

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
            .switch("unspanned", "remove the labels from the error", Some('u'))
    }

    fn description(&self) -> &str {
        "Create an error."
    }

    fn extra_description(&self) -> &str {
        "
Use either as a command with an `error_struct` or string as an input. The
`error_struct` is detailed below:

  * `msg: string` (required) 
  * `code: string`
  * `labels: table<error_label>`
  * `help: string`
  * `url: string`
  * `inner: table<error_struct>`

The `error_label` should contain the following keys:

  * `text: string` (required for each label)
  * `span: record<start: int end: int>`

Errors can be chained together using the `inner` key, and multiple spans can be
specified to give more detailed error outputs.

If a string is passed it will be the `msg` part of the `error_struct`.
"
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a simple error",
                example: "error make 'my error message'",
                result: None,
            },
            Example {
                description: "The same but with an error_struct",
                example: "error make {msg: 'my error message'}",
                result: None,
            },
            Example {
                description: "A complex error utilizing spans and inners",
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
                description: "Chain errors",
                example: r#"try {error make "foo"} catch {error make "bar"}"#,
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
        let value = match call.req(engine_state, stack, 0) {
            Ok(v @ (Value::Record { .. } | Value::String { .. })) => v,
            _ => Value::string("originates from here", call.head),
            // Err(e) => e.into_value(call.head),
        };
        let show_labels: bool = !call.has_flag(engine_state, stack, "unspanned")?;

        let inners = match ErrorInfo::from_value(input.into_value(call.head)?) {
            Ok(v) => vec![v.into_value(call.head)],
            Err(_) => vec![],
        };

        Err(match value {
            Value::String { val, .. } => ErrorInfo {
                msg: val,
                inner: inners,
                ..ErrorInfo::default()
            }
            .labeled(call.head, show_labels),
            Value::Record {
                val, internal_span, ..
            } => {
                let mut ei = ErrorInfo::from_value((*val).clone().into_value(internal_span))?;
                ei.inner = [ei.inner, inners].concat();

                ei.labeled(internal_span, show_labels)
            }
            Value::Error { error, .. } => *error,
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

        match self {
            ei @ ErrorInfo { raw: None, .. } => LabeledError {
                labels: match (show_labels, ei.clone().labels().as_slice()) {
                    (true, []) => vec![ErrorLabel {
                        text: "".into(),
                        span,
                    }],
                    (true, labels) => labels.to_vec(),
                    (false, _) => vec![],
                }
                .into_iter()
                .map(|l| l.into())
                .collect::<Vec<_>>()
                .into(),
                msg: ei.msg,
                code: ei.code,
                url: ei.url,
                help: ei.help,
                inner: inner.into(),
            }
            .into(),
            ErrorInfo { raw: Some(v), .. } => ShellError::from_value(v).unwrap_or_else(|e| e),
        }
    }
}

// impl Into<ShellError> for ErrorInfo {

// }
