use crate::prelude::*;

pub struct InputStream {
    crate values: BoxStream<'static, Spanned<Value>>,
}

impl InputStream {
    pub fn into_vec(self) -> impl Future<Output = Vec<Spanned<Value>>> {
        self.values.collect()
    }

    pub fn from_stream(input: impl Stream<Item = Spanned<Value>> + Send + 'static) -> InputStream {
        InputStream {
            values: input.boxed(),
        }
    }
}

impl From<BoxStream<'static, Spanned<Value>>> for InputStream {
    fn from(input: BoxStream<'static, Spanned<Value>>) -> InputStream {
        InputStream { values: input }
    }
}

impl From<VecDeque<Spanned<Value>>> for InputStream {
    fn from(input: VecDeque<Spanned<Value>>) -> InputStream {
        InputStream {
            values: input.boxed(),
        }
    }
}

impl From<Vec<Spanned<Value>>> for InputStream {
    fn from(input: Vec<Spanned<Value>>) -> InputStream {
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
    pub fn empty() -> OutputStream {
        let v: VecDeque<ReturnValue> = VecDeque::new();
        v.into()
    }

    pub fn one(item: impl Into<ReturnValue>) -> OutputStream {
        let v: VecDeque<ReturnValue> = VecDeque::new();
        v.push_back(item.into());
        v.into()
    }

    pub fn from_input(input: impl Stream<Item = Spanned<Value>> + Send + 'static) -> OutputStream {
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

impl From<BoxStream<'static, Spanned<Value>>> for OutputStream {
    fn from(input: BoxStream<'static, Spanned<Value>>) -> OutputStream {
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

impl From<VecDeque<Spanned<Value>>> for OutputStream {
    fn from(input: VecDeque<Spanned<Value>>) -> OutputStream {
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

impl From<Vec<Spanned<Value>>> for OutputStream {
    fn from(input: Vec<Spanned<Value>>) -> OutputStream {
        let mut list = VecDeque::default();
        list.extend(input.into_iter().map(ReturnSuccess::value));

        OutputStream {
            values: list.boxed(),
        }
    }
}
