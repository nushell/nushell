use crate::*;
use std::{cell::RefCell, fmt::Debug, rc::Rc};

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
            let mut iter = self.0.borrow_mut();
            iter.next()
        }
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
