use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::*;

#[derive(Clone)]
pub struct RowStream(Rc<RefCell<dyn Iterator<Item = Vec<Value>>>>);

impl RowStream {
    pub fn into_string(self, headers: Vec<String>) -> String {
        format!(
            "[{}]\n[{}]",
            headers
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            self.map(|x: Vec<Value>| {
                x.into_iter()
                    .map(|x| x.into_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            })
            .collect::<Vec<String>>()
            .join("\n")
        )
    }
}

impl Debug for RowStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueStream").finish()
    }
}

impl Iterator for RowStream {
    type Item = Vec<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        {
            let mut iter = self.0.borrow_mut();
            iter.next()
        }
    }
}

pub trait IntoRowStream {
    fn into_row_stream(self) -> RowStream;
}

impl IntoRowStream for Vec<Vec<Value>> {
    fn into_row_stream(self) -> RowStream {
        RowStream(Rc::new(RefCell::new(self.into_iter())))
    }
}
