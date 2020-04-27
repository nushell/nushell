use crate::value::{UntaggedValue, Value};

pub enum RowValueIter<'a> {
    Empty,
    Entries(indexmap::map::Iter<'a, String, Value>),
}

pub enum TableValueIter<'a> {
    Empty,
    Entries(std::slice::Iter<'a, Value>),
}

impl<'a> Iterator for RowValueIter<'a> {
    type Item = (&'a String, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RowValueIter::Empty => None,
            RowValueIter::Entries(iter) => iter.next(),
        }
    }
}

impl<'a> Iterator for TableValueIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TableValueIter::Empty => None,
            TableValueIter::Entries(iter) => iter.next(),
        }
    }
}

pub fn table_entries(value: &Value) -> TableValueIter<'_> {
    match &value.value {
        UntaggedValue::Table(t) => TableValueIter::Entries(t.iter()),
        _ => TableValueIter::Empty,
    }
}

pub fn row_entries(value: &Value) -> RowValueIter<'_> {
    match &value.value {
        UntaggedValue::Row(o) => {
            let iter = o.entries.iter();
            RowValueIter::Entries(iter)
        }
        _ => RowValueIter::Empty,
    }
}
