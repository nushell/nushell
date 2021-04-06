use crate::prelude::*;
use nu_protocol::{ReturnSuccess, ReturnValue, Value};
use std::iter::IntoIterator;

pub struct OutputStream {
    pub values: Box<dyn Iterator<Item = ReturnValue> + Send + Sync>,
}

impl Iterator for OutputStream {
    type Item = ReturnValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.next()
    }
}

impl OutputStream {
    pub fn new(values: impl Iterator<Item = ReturnValue> + Send + Sync + 'static) -> OutputStream {
        OutputStream {
            values: Box::new(values),
        }
    }

    pub fn empty() -> OutputStream {
        let v: VecDeque<ReturnValue> = VecDeque::new();
        v.into()
    }

    pub fn one(item: impl Into<ReturnValue>) -> OutputStream {
        let item = item.into();
        OutputStream {
            values: Box::new(std::iter::once(item)),
        }
    }

    pub fn from_input(input: impl Iterator<Item = Value> + Send + Sync + 'static) -> OutputStream {
        OutputStream {
            values: Box::new(input.map(ReturnSuccess::value)),
        }
    }

    pub fn drain_vec(&mut self) -> Vec<ReturnValue> {
        let mut output = vec![];
        while let Some(x) = self.values.next() {
            output.push(x);
        }
        output
    }
}

impl From<InputStream> for OutputStream {
    fn from(input: InputStream) -> OutputStream {
        OutputStream {
            values: Box::new(input.into_iter().map(ReturnSuccess::value)),
        }
    }
}

// impl From<impl Iterator<Item = Value> + Send + Sync + 'static> for OutputStream {
//     fn from(input: impl Iterator<Item = Value> + Send + Sync + 'static) -> OutputStream {
//         OutputStream {
//             values: Box::new(input.map(ReturnSuccess::value)),
//         }
//     }
// }

// impl From<BoxStream<'static, ReturnValue>> for OutputStream {
//     fn from(input: BoxStream<'static, ReturnValue>) -> OutputStream {
//         OutputStream { values: input }
//     }
// }

impl From<VecDeque<ReturnValue>> for OutputStream {
    fn from(input: VecDeque<ReturnValue>) -> OutputStream {
        OutputStream {
            values: Box::new(input.into_iter()),
        }
    }
}

impl From<VecDeque<Value>> for OutputStream {
    fn from(input: VecDeque<Value>) -> OutputStream {
        let stream = input.into_iter().map(ReturnSuccess::value);
        OutputStream {
            values: Box::new(stream),
        }
    }
}

impl From<Vec<ReturnValue>> for OutputStream {
    fn from(input: Vec<ReturnValue>) -> OutputStream {
        OutputStream {
            values: Box::new(input.into_iter()),
        }
    }
}

impl From<Vec<Value>> for OutputStream {
    fn from(input: Vec<Value>) -> OutputStream {
        let stream = input.into_iter().map(ReturnSuccess::value);
        OutputStream {
            values: Box::new(stream),
        }
    }
}
