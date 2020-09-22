use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use nu_protocol::RangeInclusion;
use nu_protocol::{format_primitive, ColumnPath, Dictionary, Primitive, UntaggedValue, Value};
use nu_source::{b, DebugDocBuilder, PrettyDebug};
use num_bigint::BigInt;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct InlineRange {
    from: (InlineShape, RangeInclusion),
    to: (InlineShape, RangeInclusion),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InlineShape {
    Nothing,
    Int(BigInt),
    Decimal(BigDecimal),
    Range(Box<InlineRange>),
    Bytesize(u64),
    String(String),
    Line(String),
    ColumnPath(ColumnPath),
    Pattern(String),
    Boolean(bool),
    Date(DateTime<Utc>),
    Duration(BigInt),
    Path(PathBuf),
    Binary(usize),

    Row(BTreeMap<Column, InlineShape>),
    Table(Vec<InlineShape>),

    // TODO: Block arguments
    Block,
    // TODO: Error type
    Error,

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

pub struct FormatInlineShape {
    shape: InlineShape,
    column: Option<Column>,
}

impl InlineShape {
    pub fn from_primitive(primitive: &Primitive) -> InlineShape {
        match primitive {
            Primitive::Nothing => InlineShape::Nothing,
            Primitive::Int(int) => InlineShape::Int(int.clone()),
            Primitive::Range(range) => {
                let (left, left_inclusion) = &range.from;
                let (right, right_inclusion) = &range.to;

                InlineShape::Range(Box::new(InlineRange {
                    from: (InlineShape::from_primitive(left), *left_inclusion),
                    to: (InlineShape::from_primitive(right), *right_inclusion),
                }))
            }
            Primitive::Decimal(decimal) => InlineShape::Decimal(decimal.clone()),
            Primitive::Filesize(bytesize) => InlineShape::Bytesize(*bytesize),
            Primitive::String(string) => InlineShape::String(string.clone()),
            Primitive::Line(string) => InlineShape::Line(string.clone()),
            Primitive::ColumnPath(path) => InlineShape::ColumnPath(path.clone()),
            Primitive::Pattern(pattern) => InlineShape::Pattern(pattern.clone()),
            Primitive::Boolean(boolean) => InlineShape::Boolean(*boolean),
            Primitive::Date(date) => InlineShape::Date(*date),
            Primitive::Duration(duration) => InlineShape::Duration(duration.clone()),
            Primitive::Path(path) => InlineShape::Path(path.clone()),
            Primitive::Binary(b) => InlineShape::Binary(b.len()),
            Primitive::BeginningOfStream => InlineShape::BeginningOfStream,
            Primitive::EndOfStream => InlineShape::EndOfStream,
        }
    }

    pub fn from_dictionary(dictionary: &Dictionary) -> InlineShape {
        let mut map = BTreeMap::new();

        for (key, value) in dictionary.entries.iter() {
            let column = Column::String(key.clone());
            map.insert(column, InlineShape::from_value(value));
        }

        InlineShape::Row(map)
    }

    pub fn from_table<'a>(table: impl IntoIterator<Item = &'a Value>) -> InlineShape {
        let mut vec = vec![];

        for item in table.into_iter() {
            vec.push(InlineShape::from_value(item))
        }

        InlineShape::Table(vec)
    }

    pub fn from_value<'a>(value: impl Into<&'a UntaggedValue>) -> InlineShape {
        match value.into() {
            UntaggedValue::Primitive(p) => InlineShape::from_primitive(p),
            UntaggedValue::Row(row) => InlineShape::from_dictionary(row),
            UntaggedValue::Table(table) => InlineShape::from_table(table.iter()),
            UntaggedValue::Error(_) => InlineShape::Error,
            UntaggedValue::Block(_) => InlineShape::Block,
        }
    }

    #[allow(unused)]
    pub fn format_for_column(self, column: impl Into<Column>) -> FormatInlineShape {
        FormatInlineShape {
            shape: self,
            column: Some(column.into()),
        }
    }

    pub fn format(self) -> FormatInlineShape {
        FormatInlineShape {
            shape: self,
            column: None,
        }
    }
}

impl PrettyDebug for FormatInlineShape {
    fn pretty(&self) -> DebugDocBuilder {
        let column = &self.column;

        match &self.shape {
            InlineShape::Nothing => b::blank(),
            InlineShape::Int(int) => b::primitive(format!("{}", int)),
            InlineShape::Decimal(decimal) => {
                b::description(format_primitive(&Primitive::Decimal(decimal.clone()), None))
            }
            InlineShape::Range(range) => {
                let (left, left_inclusion) = &range.from;
                let (right, right_inclusion) = &range.to;

                let op = match (left_inclusion, right_inclusion) {
                    (RangeInclusion::Inclusive, RangeInclusion::Inclusive) => "..",
                    (RangeInclusion::Inclusive, RangeInclusion::Exclusive) => "..<",
                    _ => unimplemented!(
                        "No syntax for ranges that aren't inclusive on the left and exclusive \
                         or inclusive on the right"
                    ),
                };

                left.clone().format().pretty() + b::operator(op) + right.clone().format().pretty()
            }
            InlineShape::Bytesize(bytesize) => {
                let byte = byte_unit::Byte::from_bytes(*bytesize as u128);

                let byte = byte.get_appropriate_unit(false);

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => {
                        (b::primitive(format!("{}", byte.get_value())) + b::space() + b::kind("B"))
                            .group()
                    }
                    _ => b::primitive(byte.format(1)),
                }
            }
            InlineShape::String(string) => b::primitive(string),
            InlineShape::Line(string) => b::primitive(string),
            InlineShape::ColumnPath(path) => {
                b::intersperse(path.iter().map(|member| member.pretty()), b::keyword("."))
            }
            InlineShape::Pattern(pattern) => b::primitive(pattern),
            InlineShape::Boolean(boolean) => b::primitive(
                match (boolean, column) {
                    (true, None) => "Yes",
                    (false, None) => "No",
                    (true, Some(Column::String(s))) if !s.is_empty() => s,
                    (false, Some(Column::String(s))) if !s.is_empty() => "",
                    (true, Some(_)) => "Yes",
                    (false, Some(_)) => "No",
                }
                .to_owned(),
            ),
            InlineShape::Date(date) => b::primitive(nu_protocol::format_date(date)),
            InlineShape::Duration(duration) => b::description(format_primitive(
                &Primitive::Duration(duration.clone()),
                None,
            )),
            InlineShape::Path(path) => b::primitive(path.display()),
            InlineShape::Binary(length) => b::opaque(format!("<binary: {} bytes>", length)),
            InlineShape::Row(row) => b::delimit(
                "[",
                b::kind("row")
                    + b::space()
                    + if row.keys().len() <= 6 {
                        b::intersperse(
                            row.keys().map(|key| match key {
                                Column::String(string) => b::description(string),
                                Column::Value => b::blank(),
                            }),
                            b::space(),
                        )
                    } else {
                        b::description(format!("{} columns", row.keys().len()))
                    },
                "]",
            )
            .group(),
            InlineShape::Table(rows) => b::delimit(
                "[",
                b::kind("table")
                    + b::space()
                    + b::primitive(rows.len())
                    + b::space()
                    + b::description("rows"),
                "]",
            )
            .group(),
            InlineShape::Block => b::opaque("block"),
            InlineShape::Error => b::error("error"),
            InlineShape::BeginningOfStream => b::blank(),
            InlineShape::EndOfStream => b::blank(),
        }
    }
}

pub trait GroupedValue: Debug + Clone {
    type Item;

    fn new() -> Self;
    fn merge(&mut self, value: Self::Item);
}

impl GroupedValue for Vec<(usize, usize)> {
    type Item = usize;

    fn new() -> Vec<(usize, usize)> {
        vec![]
    }

    fn merge(&mut self, new_value: usize) {
        match self.last_mut() {
            Some(value) if value.1 == new_value - 1 => {
                value.1 += 1;
            }

            _ => self.push((new_value, new_value)),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Column {
    String(String),
    Value,
}

impl Into<Column> for String {
    fn into(self) -> Column {
        Column::String(self)
    }
}

impl Into<Column> for &String {
    fn into(self) -> Column {
        Column::String(self.clone())
    }
}

impl Into<Column> for &str {
    fn into(self) -> Column {
        Column::String(self.to_string())
    }
}
