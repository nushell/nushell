use crate::prelude::*;
use futures::stream::iter;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, ReturnValue, UntaggedValue, Value};
use nu_source::{Tagged, TaggedItem};

pub struct InputStream {
    pub(crate) values: BoxStream<'static, Value>,
}

impl InputStream {
    pub fn empty() -> InputStream {
        vec![UntaggedValue::nothing().into_value(Tag::unknown())].into()
    }

    pub fn into_vec(self) -> impl Future<Output = Vec<Value>> {
        self.values.collect()
    }

    pub fn drain_vec(&mut self) -> impl Future<Output = Vec<Value>> {
        let mut values: BoxStream<'static, Value> = iter(VecDeque::new()).boxed();
        std::mem::swap(&mut values, &mut self.values);

        values.collect()
    }

    pub fn from_stream(input: impl Stream<Item = Value> + Send + 'static) -> InputStream {
        InputStream {
            values: input.boxed(),
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
                    value: UntaggedValue::Primitive(Primitive::Line(s)),
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
                Some(Value { tag: value_tag, .. }) => {
                    return Err(ShellError::labeled_error_with_secondary(
                        "Expected a string from pipeline",
                        "requires string input",
                        tag,
                        "value originates from here",
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
                    value: v,
                }) => {
                    println!("{:?}", v);
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
        InputStream { values: input }
    }
}

impl From<VecDeque<Value>> for InputStream {
    fn from(input: VecDeque<Value>) -> InputStream {
        InputStream {
            values: futures::stream::iter(input).boxed(),
        }
    }
}

impl From<Vec<Value>> for InputStream {
    fn from(input: Vec<Value>) -> InputStream {
        InputStream {
            values: futures::stream::iter(input).boxed(),
        }
    }
}

pub struct OutputStream {
    pub(crate) values: BoxStream<'static, ReturnValue>,
}

impl OutputStream {
    pub fn new(values: impl Stream<Item = ReturnValue> + Send + 'static) -> OutputStream {
        OutputStream {
            values: values.boxed(),
        }
    }

    pub fn empty() -> OutputStream {
        let v: VecDeque<ReturnValue> = VecDeque::new();
        v.into()
    }

    pub fn one(item: impl Into<ReturnValue>) -> OutputStream {
        let mut v: VecDeque<ReturnValue> = VecDeque::new();
        v.push_back(item.into());
        v.into()
    }

    pub fn from_input(input: impl Stream<Item = Value> + Send + 'static) -> OutputStream {
        OutputStream {
            values: input.map(ReturnSuccess::value).boxed(),
        }
    }

    pub fn drain_vec(&mut self) -> impl Future<Output = Vec<ReturnValue>> {
        let mut values: BoxStream<'static, ReturnValue> = iter(VecDeque::new()).boxed();
        std::mem::swap(&mut values, &mut self.values);

        values.collect()
    }
}

impl Stream for OutputStream {
    type Item = ReturnValue;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        Stream::poll_next(std::pin::Pin::new(&mut self.values), cx)
    }
}

impl From<InputStream> for OutputStream {
    fn from(input: InputStream) -> OutputStream {
        OutputStream {
            values: input.values.map(ReturnSuccess::value).boxed(),
        }
    }
}

impl From<BoxStream<'static, Value>> for OutputStream {
    fn from(input: BoxStream<'static, Value>) -> OutputStream {
        OutputStream {
            values: input.map(ReturnSuccess::value).boxed(),
        }
    }
}

impl From<BoxStream<'static, ReturnValue>> for OutputStream {
    fn from(input: BoxStream<'static, ReturnValue>) -> OutputStream {
        OutputStream { values: input }
    }
}

impl From<VecDeque<ReturnValue>> for OutputStream {
    fn from(input: VecDeque<ReturnValue>) -> OutputStream {
        OutputStream {
            values: futures::stream::iter(input).boxed(),
        }
    }
}

impl From<VecDeque<Value>> for OutputStream {
    fn from(input: VecDeque<Value>) -> OutputStream {
        let stream = input.into_iter().map(ReturnSuccess::value);
        OutputStream {
            values: futures::stream::iter(stream).boxed(),
        }
    }
}

impl From<Vec<ReturnValue>> for OutputStream {
    fn from(input: Vec<ReturnValue>) -> OutputStream {
        OutputStream {
            values: futures::stream::iter(input).boxed(),
        }
    }
}

impl From<Vec<Value>> for OutputStream {
    fn from(input: Vec<Value>) -> OutputStream {
        let stream = input.into_iter().map(ReturnSuccess::value);
        OutputStream {
            values: futures::stream::iter(stream).boxed(),
        }
    }
}
