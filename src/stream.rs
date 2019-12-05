use crate::prelude::*;
use nu_protocol::{ReturnSuccess, ReturnValue, UntaggedValue, Value};

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
        let mut values: BoxStream<'static, Value> = VecDeque::new().boxed();
        std::mem::swap(&mut values, &mut self.values);

        values.collect()
    }

    pub fn from_stream(input: impl Stream<Item = Value> + Send + 'static) -> InputStream {
        InputStream {
            values: input.boxed(),
        }
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
            values: input.boxed(),
        }
    }
}

impl From<Vec<Value>> for InputStream {
    fn from(input: Vec<Value>) -> InputStream {
        let mut list = VecDeque::default();
        list.extend(input);

        InputStream {
            values: list.boxed(),
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
        let mut values: BoxStream<'static, ReturnValue> = VecDeque::new().boxed();
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
            values: input.boxed(),
        }
    }
}

impl From<VecDeque<Value>> for OutputStream {
    fn from(input: VecDeque<Value>) -> OutputStream {
        OutputStream {
            values: input
                .into_iter()
                .map(ReturnSuccess::value)
                .collect::<VecDeque<ReturnValue>>()
                .boxed(),
        }
    }
}

impl From<Vec<ReturnValue>> for OutputStream {
    fn from(input: Vec<ReturnValue>) -> OutputStream {
        let mut list = VecDeque::default();
        list.extend(input);

        OutputStream {
            values: list.boxed(),
        }
    }
}

impl From<Vec<Value>> for OutputStream {
    fn from(input: Vec<Value>) -> OutputStream {
        let mut list = VecDeque::default();
        list.extend(input.into_iter().map(ReturnSuccess::value));

        OutputStream {
            values: list.boxed(),
        }
    }
}
