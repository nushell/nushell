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
  * `labels: oneof<table, record>`
  * `help: string`
  * `url: string`
  * `inner: table`

The `labels` key allow for placing arrows to points in the code, optionally
using `span` to choose where it points (see `metadata`). `label` can be a table
(list of records) or a single record. There is an example of both in the
examples. Each record has the following structure:

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
            labels: {  # optional, a table or single record
                text: "fish right here"  # Required if $.labels exists
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
#[derive(Debug, Default, Clone, FromValue, IntoValue)]
struct Error {
    msg: String,
    labels: Option<Labels>,
    // TODO: Deprecate and clean up the parsing
    label: Option<Labels>,
    inner: Option<Vec<Value>>,
    help: Option<String>,
    url: Option<String>,
    code: Option<String>,
}

impl Error {
    pub fn combined_labels(&self, span: Option<Span>) -> Vec<Label> {
        let included = [
            self.labels.clone().unwrap_or_default().list,
            self.label.clone().unwrap_or_default().list,
        ]
        .concat();
        if included.is_empty() {
            vec![Label {
                text: "originates from here".into(),
                span,
            }]
        } else {
            included
        }
    }
}

// Labels are parse separately because they could be vectors or single values.
#[derive(Debug, Default, Clone, FromValue, IntoValue)]
struct Label {
    text: String,
    span: Option<Span>,
}

// Optional list or singleton label
#[derive(Debug, Default, Clone, IntoValue)]
struct Labels {
    list: Vec<Label>,
}

impl FromValue for Labels {
    fn from_value(v: Value) -> std::result::Result<Self, ShellError> {
        match v.get_type() {
            Type::Record(_) => match Label::from_value(v) {
                Ok(o) => Ok(Self { list: vec![o] }),
                Err(o) => Err(o),
            },
            Type::Table(_) => match Vec::<Label>::from_value(v) {
                Ok(o) => Ok(Self { list: o }),
                Err(o) => Err(o),
            },
            _ => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: v.get_type().to_string(),
                span: v.span(),
                help: None,
            }),
        }
    }
}

fn make_other_error(value: &Value, throw_span: Option<Span>) -> ShellError {
    match Error::from_value(value.clone()) {
        Err(e) => e,
        Ok(v) => {
            // Main error that will be returned
            let mut error = LabeledError::new(v.msg.clone());
            if let Some(ts) = throw_span {
                for label in v.combined_labels(throw_span) {
                    error = error.with_label(label.text, label.span.unwrap_or(ts));
                }
            }
            // Recurse into the inner errors
            for inner in v.inner.unwrap_or_default() {
                error = error.with_inner(make_other_error(&inner, Some(inner.span())));
            }
            // TODO: This could be enabled before `label` is set to be deprecated
            // if !v.label.unwrap_or_default().list.is_empty() {
            //     error = error.with_inner(make_other_error(
            //         &Error {
            //             msg: "`label` is going to be deprecated. Use `labels` instead.".into(),
            //             ..Error::default()
            //         }
            //         .into_value(value.span()),
            //         throw_span,
            //     ));
            // };
            error.code = v.code;
            error.help = v.help;
            error.url = v.url;
            error.into()
        }
    }
}
