use crate::prelude::*;
use nu_protocol::{ReturnSuccess, ReturnValue, Value};
use std::iter::IntoIterator;

pub type OutputStream = InputStream;

pub struct ActionStream {
    pub values: Box<dyn Iterator<Item = ReturnValue> + Send + Sync>,
}

impl Iterator for ActionStream {
    type Item = ReturnValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.next()
    }
}

impl ActionStream {
    pub fn new(values: impl Iterator<Item = ReturnValue> + Send + Sync + 'static) -> ActionStream {
        ActionStream {
            values: Box::new(values),
        }
    }

    pub fn empty() -> ActionStream {
        ActionStream {
            values: Box::new(std::iter::empty()),
        }
    }

    pub fn one(item: impl Into<ReturnValue>) -> ActionStream {
        let item = item.into();
        ActionStream {
            values: Box::new(std::iter::once(item)),
        }
    }

    pub fn from_input(input: impl Iterator<Item = Value> + Send + Sync + 'static) -> ActionStream {
        ActionStream {
            values: Box::new(input.map(ReturnSuccess::value)),
        }
    }

    pub fn drain_vec(&mut self) -> Vec<ReturnValue> {
        let mut output = vec![];

        for x in &mut self.values {
            output.push(x);
        }

        output
    }
}

impl From<InputStream> for ActionStream {
    fn from(input: InputStream) -> ActionStream {
        ActionStream {
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

impl From<VecDeque<ReturnValue>> for ActionStream {
    fn from(input: VecDeque<ReturnValue>) -> ActionStream {
        ActionStream {
            values: Box::new(input.into_iter()),
        }
    }
}

impl From<VecDeque<Value>> for ActionStream {
    fn from(input: VecDeque<Value>) -> ActionStream {
        let stream = input.into_iter().map(ReturnSuccess::value);
        ActionStream {
            values: Box::new(stream),
        }
    }
}

impl From<Vec<ReturnValue>> for ActionStream {
    fn from(input: Vec<ReturnValue>) -> ActionStream {
        ActionStream {
            values: Box::new(input.into_iter()),
        }
    }
}

impl From<Vec<Value>> for ActionStream {
    fn from(input: Vec<Value>) -> ActionStream {
        let stream = input.into_iter().map(ReturnSuccess::value);
        ActionStream {
            values: Box::new(stream),
        }
    }
}
