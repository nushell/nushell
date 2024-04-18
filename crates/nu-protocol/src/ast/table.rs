use super::Expression;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, iter::FusedIterator};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    exprs: Box<[Box<[Expression]>]>,
}

impl Table {
    pub fn from_raw(exprs: Box<[Box<[Expression]>]>) -> Self {
        debug_assert!(!exprs.is_empty());
        debug_assert!(exprs.windows(2).all(|chunk| {
            let [a, b] = chunk else { return false };
            a.len() == b.len()
        }));
        Self { exprs }
    }

    pub fn columns(&self) -> &[Expression] {
        &self.exprs[0]
    }

    pub fn columns_mut(&mut self) -> &mut [Expression] {
        &mut self.exprs[0]
    }

    pub fn rows(&self) -> Rows {
        Rows {
            iter: self.exprs[1..].iter(),
        }
    }

    pub fn rows_mut(&mut self) -> RowsMut {
        RowsMut {
            iter: self.exprs[1..].iter_mut(),
        }
    }

    pub fn into_rows(self) -> IntoRows {
        IntoRows {
            iter: self.exprs.into_vec().into_iter().skip(1),
        }
    }
}

pub struct Rows<'a> {
    iter: std::slice::Iter<'a, Box<[Expression]>>,
}

impl<'a> Iterator for Rows<'a> {
    type Item = &'a [Expression];

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(AsRef::as_ref)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Rows<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(AsRef::as_ref)
    }
}

impl ExactSizeIterator for Rows<'_> {}

impl FusedIterator for Rows<'_> {}

pub struct RowsMut<'a> {
    iter: std::slice::IterMut<'a, Box<[Expression]>>,
}

impl<'a> Iterator for RowsMut<'a> {
    type Item = &'a mut [Expression];

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(AsMut::as_mut)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for RowsMut<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(AsMut::as_mut)
    }
}

impl ExactSizeIterator for RowsMut<'_> {}

impl FusedIterator for RowsMut<'_> {}

pub struct IntoRows {
    iter: std::iter::Skip<std::vec::IntoIter<Box<[Expression]>>>,
}

impl Iterator for IntoRows {
    type Item = Vec<Expression>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Into::into)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for IntoRows {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(Into::into)
    }
}

impl ExactSizeIterator for IntoRows {}

impl FusedIterator for IntoRows {}

// impl Debug for Table {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("Table")
//             .field("head", &self.columns())
//             .field("rows", &self.rows_chunked())
//             .finish()
//     }
// }
