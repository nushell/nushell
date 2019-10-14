use crate::prelude::*;

pub struct InputStream {
    pub(crate) values: BoxStream<'static, Tagged<Value>>,
}

impl InputStream {
    pub fn into_vec(self) -> impl Future<Output = Vec<Tagged<Value>>> {
        self.values.collect()
    }

    pub fn drain_vec(&mut self) -> impl Future<Output = Vec<Tagged<Value>>> {
        let mut values: BoxStream<'static, Tagged<Value>> = VecDeque::new().boxed();
        std::mem::swap(&mut values, &mut self.values);

        values.collect()
    }

    pub fn from_stream(input: impl Stream<Item = Tagged<Value>> + Send + 'static) -> InputStream {
        InputStream {
            values: input.boxed(),
        }
    }
}

impl Stream for InputStream {
    type Item = Tagged<Value>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        Stream::poll_next(std::pin::Pin::new(&mut self.values), cx)
    }
}

impl From<BoxStream<'static, Tagged<Value>>> for InputStream {
    fn from(input: BoxStream<'static, Tagged<Value>>) -> InputStream {
        InputStream { values: input }
    }
}

impl From<VecDeque<Tagged<Value>>> for InputStream {
    fn from(input: VecDeque<Tagged<Value>>) -> InputStream {
        InputStream {
            values: input.boxed(),
        }
    }
}

impl From<Vec<Tagged<Value>>> for InputStream {
    fn from(input: Vec<Tagged<Value>>) -> InputStream {
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

    pub fn from_input(input: impl Stream<Item = Tagged<Value>> + Send + 'static) -> OutputStream {
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

impl From<BoxStream<'static, Tagged<Value>>> for OutputStream {
    fn from(input: BoxStream<'static, Tagged<Value>>) -> OutputStream {
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

impl From<VecDeque<Tagged<Value>>> for OutputStream {
    fn from(input: VecDeque<Tagged<Value>>) -> OutputStream {
        OutputStream {
            values: input
                .into_iter()
                .map(|i| ReturnSuccess::value(i))
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

impl From<Vec<Tagged<Value>>> for OutputStream {
    fn from(input: Vec<Tagged<Value>>) -> OutputStream {
        let mut list = VecDeque::default();
        list.extend(input.into_iter().map(ReturnSuccess::value));

        OutputStream {
            values: list.boxed(),
        }
    }
}
