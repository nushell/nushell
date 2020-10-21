// use crate::config::{Conf, NuConfig};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use indexmap::map::IndexMap;
use nu_protocol::RangeInclusion;
use nu_protocol::{format_primitive, ColumnPath, Dictionary, Primitive, UntaggedValue, Value};
use nu_source::{b, DebugDocBuilder, PrettyDebug, Tag};
use num_bigint::BigInt;
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct InlineRange {
    from: (InlineShape, RangeInclusion),
    to: (InlineShape, RangeInclusion),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
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

    Row(Row),
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
        let mut map = IndexMap::new();

        for (key, value) in dictionary.entries.iter() {
            let column = Column::String(key.clone());
            map.insert(column, InlineShape::from_value(value));
        }

        InlineShape::Row(Row { map })
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
                // get the config value, if it doesn't exist make it 'auto' so it works how it originally did
                let filesize_format_var = crate::config::config(Tag::unknown())
                    .expect("unabled to get the config.toml file")
                    .get("filesize_format")
                    .map(|val| val.convert_to_string().to_ascii_lowercase())
                    .unwrap_or_else(|| "auto".to_string());
                // if there is a value, match it to one of the valid values for byte units
                let filesize_format = match filesize_format_var.as_str() {
                    "b" => (byte_unit::ByteUnit::B, ""),
                    "kb" => (byte_unit::ByteUnit::KB, ""),
                    "kib" => (byte_unit::ByteUnit::KiB, ""),
                    "mb" => (byte_unit::ByteUnit::MB, ""),
                    "mib" => (byte_unit::ByteUnit::MiB, ""),
                    "gb" => (byte_unit::ByteUnit::GB, ""),
                    "gib" => (byte_unit::ByteUnit::GiB, ""),
                    "tb" => (byte_unit::ByteUnit::TB, ""),
                    "tib" => (byte_unit::ByteUnit::TiB, ""),
                    "pb" => (byte_unit::ByteUnit::PB, ""),
                    "pib" => (byte_unit::ByteUnit::PiB, ""),
                    "eb" => (byte_unit::ByteUnit::EB, ""),
                    "eib" => (byte_unit::ByteUnit::EiB, ""),
                    "zb" => (byte_unit::ByteUnit::ZB, ""),
                    "zib" => (byte_unit::ByteUnit::ZiB, ""),
                    _ => (byte_unit::ByteUnit::B, "auto"),
                };

                let byte = byte_unit::Byte::from_bytes(*bytesize as u128);
                let byte =
                    if filesize_format.0 == byte_unit::ByteUnit::B && filesize_format.1 == "auto" {
                        byte.get_appropriate_unit(false)
                    } else {
                        byte.get_adjusted_unit(filesize_format.0)
                    };

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => {
                        let locale_byte = byte.get_value() as u64;
                        (b::primitive(locale_byte.to_formatted_string(&Locale::en))
                            + b::space()
                            + b::kind("B"))
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
                    + if row.map.keys().len() <= 6 {
                        b::intersperse(
                            row.map.keys().map(|key| match key {
                                Column::String(string) => b::description(string),
                                Column::Value => b::blank(),
                            }),
                            b::space(),
                        )
                    } else {
                        b::description(format!("{} columns", row.map.keys().len()))
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
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

/// A shape representation of the type of a row
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    map: IndexMap<Column, InlineShape>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Row {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.map.clone();
        entries.sort_keys();
        entries.keys().collect::<Vec<&Column>>().hash(state);
        entries.values().collect::<Vec<&InlineShape>>().hash(state);
    }
}

impl PartialOrd for Row {
    fn partial_cmp(&self, other: &Row) -> Option<Ordering> {
        let this: Vec<&Column> = self.map.keys().collect();
        let that: Vec<&Column> = other.map.keys().collect();

        if this != that {
            return this.partial_cmp(&that);
        }

        let this: Vec<&InlineShape> = self.map.values().collect();
        let that: Vec<&InlineShape> = self.map.values().collect();

        this.partial_cmp(&that)
    }
}

impl Ord for Row {
    /// Compare two dictionaries for ordering
    fn cmp(&self, other: &Row) -> Ordering {
        let this: Vec<&Column> = self.map.keys().collect();
        let that: Vec<&Column> = other.map.keys().collect();

        if this != that {
            return this.cmp(&that);
        }

        let this: Vec<&InlineShape> = self.map.values().collect();
        let that: Vec<&InlineShape> = self.map.values().collect();

        this.cmp(&that)
    }
}
