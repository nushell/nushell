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
            .input_output_types(vec![
                (Type::Nothing, Type::Error),
                (Type::record(), Type::Error),
            ])
            .required(
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
            msg: "an error for foo just occured"
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
        let value: Value = call.req(engine_state, stack, 0)?;
        let show_labels: bool = !call.has_flag(engine_state, stack, "unspanned")?;

        let inners = match ErrorInfo::from_value(input.into_value(call.head)?) {
            Ok(v) => vec![Inner {
                v: Ok(v.into_value(call.head)),
            }],
            Err(_) => vec![],
        };

        Err(match value.get_type() {
            Type::String => ErrorInfo {
                msg: String::from_value(value)?,
                inner: inners,
                ..ErrorInfo::default()
            },
            _ => match ErrorInfo::from_value(value) {
                Ok(mut e) => {
                    e.inner = [e.inner, inners].concat();
                    e
                }
                Err(err) => return Err(err),
            },
        }
        .labeled(call.head, show_labels)
        .into())
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
    inner: Vec<Inner>,
    raw: Option<Value>,
}

#[derive(Debug, Clone)]
struct Inner {
    v: Result<Value, ShellError>,
}

impl FromValue for Inner {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        match v.clone().unwrap_error() {
            Ok(se) => Ok(Inner { v: Ok(se) }),
            Err(e) => match v.clone() {
                Value::Record { .. } => Ok(Inner { v: Ok(v) }),
                _ => Err(e),
            },
        }
    }
}

impl IntoValue for Inner {
    fn into_value(self, span: Span) -> Value {
        match self.v {
            Ok(value) => value,
            Err(shellerror) => Value::error(shellerror, span),
        }
    }
}

impl From<ShellError> for Inner {
    fn from(value: ShellError) -> Self {
        Self { v: Err(value) }
    }
}

impl From<LabeledError> for Inner {
    fn from(value: LabeledError) -> Self {
        Self {
            v: Err(value.into()),
        }
    }
}

impl Default for ErrorInfo {
    fn default() -> Self {
        Self {
            msg: "Error".into(),
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

impl From<LabeledError> for ErrorInfo {
    fn from(value: LabeledError) -> Self {
        Self {
            msg: value.msg,
            code: value.code,
            help: value.help,
            url: value.url,
            labels: *value.labels,
            inner: (*value.inner).into_iter().map(Inner::from).collect(),
            raw: None,
            ..Self::default()
        }
    }
}

impl From<ShellError> for ErrorInfo {
    fn from(value: ShellError) -> Self {
        let labeled: LabeledError = value.into();
        Self::from(labeled)
    }
}

fn remove_labels(mut labeled: LabeledError) -> LabeledError {
    labeled.labels = vec![].into();
    labeled.inner = Box::new((*labeled.inner).into_iter().map(remove_labels).collect());
    labeled
}

// It's funny how this looks like the old one again haha
impl ErrorInfo {
    fn labels(self) -> Vec<ErrorLabel> {
        [self.labels, self.label.map(|i| vec![i]).unwrap_or_default()].concat()
    }

    fn labeled(self, span: Span, show_labels: bool) -> LabeledError {
        // Initialize the error with the message if we have one. This will be
        // overwritten eventually.
        let mut error: LabeledError = if let Some(raw) = self.clone().raw {
            // ErrorInfo::from_value(raw).unwrap_or_default()
            match raw.unwrap_error() {
                Err(e) => e.into(),
                Ok(_) => LabeledError::new(self.msg.clone()),
            }
        } else {
            LabeledError::new(self.msg.clone())
        };

        // Gather up stuff that is in arrays
        let inners: Vec<LabeledError> = self
            .inner
            .iter()
            .map(|val| {
                match val.clone().v {
                    Ok(err) => match ErrorInfo::from_value(err) {
                        Ok(e) => e,
                        Err(e) => ErrorInfo::from(e),
                    },
                    Err(se) => ErrorInfo::from(se),
                }
                .labeled(span, show_labels)
            })
            .collect();
        let labels = match (
            show_labels,
            [self.clone().labels(), *error.labels].concat().as_slice(),
        ) {
            (false, _) => vec![],
            (true, []) => vec![ErrorLabel {
                text: "originates from here".into(),
                span,
            }],
            (true, all_labels) => all_labels
                .iter()
                .map(|l| {
                    if l.span == Span::default() {
                        ErrorLabel {
                            text: l.clone().text,
                            span,
                        }
                    } else {
                        l.clone()
                    }
                })
                .collect(),
        };
        error.msg = self.msg;
        error.url = self.url;
        error.code = self.code;
        error.help = self.help;
        error.labels = Box::new(labels);
        if show_labels {
            error.inner = [*error.inner, inners].concat().into();
        } else {
            error.inner = Vec::default().into();
        }

        if !show_labels {
            remove_labels(error)
        } else {
            error
        }
    }
}
