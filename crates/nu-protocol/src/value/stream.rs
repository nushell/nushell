use crate::*;
use std::{cell::RefCell, fmt::Debug, rc::Rc};

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct ValueStream(pub Rc<RefCell<dyn Iterator<Item = Value>>>);

impl ValueStream {
    pub fn into_string(self) -> String {
        format!(
            "[{}]",
            self.map(|x: Value| x.into_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }

    pub fn from_stream(input: impl Iterator<Item = Value> + 'static) -> ValueStream {
        ValueStream(Rc::new(RefCell::new(input)))
    }
}

impl Debug for ValueStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueStream").finish()
    }
}

impl Iterator for ValueStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        {
            self.0.borrow_mut().next()
        }
    }
}

impl Serialize for ValueStream {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // FIXME: implement these
        todo!()
    }
}

impl<'de> Deserialize<'de> for ValueStream {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // FIXME: implement these
        todo!()
    }
}

pub trait IntoValueStream {
    fn into_value_stream(self) -> ValueStream;
}

impl<T> IntoValueStream for T
where
    T: Iterator<Item = Value> + 'static,
{
    fn into_value_stream(self) -> ValueStream {
        ValueStream::from_stream(self)
    }
}
