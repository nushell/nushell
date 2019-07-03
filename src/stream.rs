use crate::prelude::*;

pub struct InputStream {
    crate values: BoxStream<'static, Value>,
}

impl InputStream {
    pub fn into_vec(self) -> impl Future<Output = Vec<Value>> {
        self.values.collect()
    }

    pub fn from_stream(input: impl Stream<Item = Value> + Send + 'static) -> InputStream {
        InputStream {
            values: input.boxed(),
        }
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
    crate values: BoxStream<'static, ReturnValue>,
}

impl OutputStream {
    pub fn from_stream(input: impl Stream<Item = ReturnValue> + Send + 'static) -> OutputStream {
        OutputStream {
            values: input.boxed(),
        }
    }

    pub fn from_input(input: impl Stream<Item = Value> + Send + 'static) -> OutputStream {
        OutputStream {
            values: input.map(ReturnSuccess::value).boxed(),
        }
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
