use crate::prelude::*;
use futures::stream::iter;
use nu_protocol::{ReturnSuccess, ReturnValue, Value};
use std::iter::IntoIterator;

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
        let item = item.into();
        futures::stream::once(async move { item }).to_output_stream()
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
            values: input.map(ReturnSuccess::value).boxed(),
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
