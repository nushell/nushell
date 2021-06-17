use crate::prelude::*;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Type, UntaggedValue, Value};
use nu_source::{HasFallibleSpan, PrettyDebug, Tag, Tagged, TaggedItem};

pub struct InputStream {
    values: Box<dyn Iterator<Item = Value> + Send + Sync>,

    // Whether or not an empty stream was explicitly requested via InputStream::empty
    empty: bool,
}

impl Iterator for InputStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.next()
    }
}

impl InputStream {
    pub fn empty() -> InputStream {
        InputStream {
            values: Box::new(std::iter::empty()),
            empty: true,
        }
    }

    pub fn one(item: impl Into<Value>) -> InputStream {
        InputStream {
            values: Box::new(std::iter::once(item.into())),
            empty: false,
        }
    }

    pub fn into_vec(self) -> Vec<Value> {
        self.values.collect()
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn drain_vec(&mut self) -> Vec<Value> {
        let mut output = vec![];
        for x in &mut self.values {
            output.push(x);
        }
        output
    }

    pub fn from_stream(input: impl Iterator<Item = Value> + Send + Sync + 'static) -> InputStream {
        InputStream {
            values: Box::new(input),
            empty: false,
        }
    }

    pub fn collect_string(mut self, tag: Tag) -> Result<Tagged<String>, ShellError> {
        let mut bytes = vec![];
        let mut value_tag = tag.clone();

        loop {
            match self.values.next() {
                Some(Value {
                    value: UntaggedValue::Primitive(Primitive::String(s)),
                    tag: value_t,
                }) => {
                    value_tag = value_t;
                    bytes.extend_from_slice(&s.into_bytes());
                }
                Some(Value {
                    value: UntaggedValue::Primitive(Primitive::Binary(b)),
                    tag: value_t,
                }) => {
                    value_tag = value_t;
                    bytes.extend_from_slice(&b);
                }
                Some(Value {
                    value: UntaggedValue::Primitive(Primitive::Nothing),
                    tag: value_t,
                }) => {
                    value_tag = value_t;
                }
                Some(Value {
                    tag: value_tag,
                    value,
                }) => {
                    return Err(ShellError::labeled_error_with_secondary(
                        "Expected a string from pipeline",
                        "requires string input",
                        tag,
                        format!(
                            "{} originates from here",
                            Type::from_value(&value).plain_string(100000)
                        ),
                        value_tag,
                    ))
                }
                None => break,
            }
        }

        match String::from_utf8(bytes) {
            Ok(s) => Ok(s.tagged(value_tag)),
            Err(_) => Err(ShellError::labeled_error_with_secondary(
                "Expected a string from pipeline",
                "requires string input",
                tag,
                "value originates from here",
                value_tag,
            )),
        }
    }

    pub fn collect_binary(mut self, tag: Tag) -> Result<Tagged<Vec<u8>>, ShellError> {
        let mut bytes = vec![];
        let mut value_tag = tag.clone();

        loop {
            match self.values.next() {
                Some(Value {
                    value: UntaggedValue::Primitive(Primitive::Binary(b)),
                    tag: value_t,
                }) => {
                    value_tag = value_t;
                    bytes.extend_from_slice(&b);
                }
                Some(Value {
                    tag: value_tag,
                    value: _,
                }) => {
                    return Err(ShellError::labeled_error_with_secondary(
                        "Expected binary from pipeline",
                        "requires binary input",
                        tag,
                        "value originates from here",
                        value_tag,
                    ));
                }
                None => break,
            }
        }

        Ok(bytes.tagged(value_tag))
    }
}

impl From<VecDeque<Value>> for InputStream {
    fn from(input: VecDeque<Value>) -> InputStream {
        InputStream {
            values: Box::new(input.into_iter()),
            empty: false,
        }
    }
}

impl From<Vec<Value>> for InputStream {
    fn from(input: Vec<Value>) -> InputStream {
        InputStream {
            values: Box::new(input.into_iter()),
            empty: false,
        }
    }
}

pub trait IntoInputStream {
    fn into_input_stream(self) -> InputStream;
}

impl<T, U> IntoInputStream for T
where
    T: Iterator<Item = U> + Send + Sync + 'static,
    U: Into<Result<nu_protocol::Value, nu_errors::ShellError>>,
{
    fn into_input_stream(self) -> InputStream {
        InputStream {
            empty: false,
            values: Box::new(self.map(|item| match item.into() {
                Ok(result) => result,
                Err(err) => match HasFallibleSpan::maybe_span(&err) {
                    Some(span) => nu_protocol::UntaggedValue::Error(err).into_value(span),
                    None => nu_protocol::UntaggedValue::Error(err).into_untagged_value(),
                },
            })),
        }
    }
}
