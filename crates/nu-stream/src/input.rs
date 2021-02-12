use crate::prelude::*;
use futures::stream::iter;
use nu_errors::ShellError;
use nu_protocol::{Primitive, Type, UntaggedValue, Value};
use nu_source::{HasFallibleSpan, PrettyDebug, Tag, Tagged, TaggedItem};

pub struct InputStream {
    values: BoxStream<'static, Value>,

    // Whether or not an empty stream was explicitly requested via InputStream::empty
    empty: bool,
}

impl InputStream {
    pub fn empty() -> InputStream {
        InputStream {
            values: futures::stream::empty().boxed(),
            empty: true,
        }
    }

    pub fn one(item: impl Into<Value>) -> InputStream {
        let mut v: VecDeque<Value> = VecDeque::new();
        v.push_back(item.into());
        v.into()
    }

    pub fn into_vec(self) -> impl Future<Output = Vec<Value>> {
        self.values.collect()
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn drain_vec(&mut self) -> impl Future<Output = Vec<Value>> {
        let mut values: BoxStream<'static, Value> = iter(VecDeque::new()).boxed();
        std::mem::swap(&mut values, &mut self.values);

        values.collect()
    }

    pub fn from_stream(input: impl Stream<Item = Value> + Send + 'static) -> InputStream {
        InputStream {
            values: input.boxed(),
            empty: false,
        }
    }

    pub async fn collect_string(mut self, tag: Tag) -> Result<Tagged<String>, ShellError> {
        let mut bytes = vec![];
        let mut value_tag = tag.clone();

        loop {
            match self.values.next().await {
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
            Ok(s) => Ok(s.tagged(value_tag.clone())),
            Err(_) => Err(ShellError::labeled_error_with_secondary(
                "Expected a string from pipeline",
                "requires string input",
                tag,
                "value originates from here",
                value_tag,
            )),
        }
    }

    pub async fn collect_binary(mut self, tag: Tag) -> Result<Tagged<Vec<u8>>, ShellError> {
        let mut bytes = vec![];
        let mut value_tag = tag.clone();

        loop {
            match self.values.next().await {
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

impl Stream for InputStream {
    type Item = Value;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        Stream::poll_next(std::pin::Pin::new(&mut self.values), cx)
    }
}

impl From<BoxStream<'static, Value>> for InputStream {
    fn from(input: BoxStream<'static, Value>) -> InputStream {
        InputStream {
            values: input,
            empty: false,
        }
    }
}

impl From<VecDeque<Value>> for InputStream {
    fn from(input: VecDeque<Value>) -> InputStream {
        InputStream {
            values: futures::stream::iter(input).boxed(),
            empty: false,
        }
    }
}

impl From<Vec<Value>> for InputStream {
    fn from(input: Vec<Value>) -> InputStream {
        InputStream {
            values: futures::stream::iter(input).boxed(),
            empty: false,
        }
    }
}

#[allow(clippy::clippy::wrong_self_convention)]
pub trait ToInputStream {
    fn to_input_stream(self) -> InputStream;
}

impl<T, U> ToInputStream for T
where
    T: Stream<Item = U> + Send + 'static,
    U: Into<Result<nu_protocol::Value, nu_errors::ShellError>>,
{
    fn to_input_stream(self) -> InputStream {
        InputStream::from_stream(self.map(|item| match item.into() {
            Ok(result) => result,
            Err(err) => match HasFallibleSpan::maybe_span(&err) {
                Some(span) => nu_protocol::UntaggedValue::Error(err).into_value(span),
                None => nu_protocol::UntaggedValue::Error(err).into_untagged_value(),
            },
        }))
    }
}
