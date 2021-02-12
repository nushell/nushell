///
/// This file describes the structural types of the nushell system.
///
/// Its primary purpose today is to identify "equivalent" values for the purpose
/// of merging rows into a single table or identify rows in a table that have the
/// same shape for reflection.
use crate::value::dict::Dictionary;
use crate::value::primitive::Primitive;
use crate::value::range::RangeInclusion;
use crate::value::{UntaggedValue, Value};
use derive_new::new;
use indexmap::map::IndexMap;
use nu_source::{DbgDocBldr, DebugDoc, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// Representation of the type of ranges
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, new)]
pub struct RangeType {
    from: (Type, RangeInclusion),
    to: (Type, RangeInclusion),
}

/// Representation of for the type of a value in Nu
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Type {
    /// A value which has no value
    Nothing,
    /// An integer-based value
    Int,
    /// A range between two values
    Range(Box<RangeType>),
    /// A decimal (floating point) value
    Decimal,
    /// A filesize in bytes
    Filesize,
    /// A string of text
    String,
    /// A line of text (a string with trailing line ending)
    Line,
    /// A path through a table
    ColumnPath,
    /// A glob pattern (like foo*)
    GlobPattern,
    /// A boolean value
    Boolean,
    /// A date value (in UTC)
    Date,
    /// A data duration value
    Duration,
    /// A filepath value
    FilePath,
    /// A binary (non-text) buffer value
    Binary,

    /// A row of data
    Row(Row),
    /// A full table of data
    Table(Vec<Type>),

    /// A block of script (TODO)
    Block,
    /// An error value (TODO)
    Error,

    /// Beginning of stream marker (used as bookend markers rather than actual values)
    BeginningOfStream,
    /// End of stream marker (used as bookend markers rather than actual values)
    EndOfStream,
}

/// A shape representation of the type of a row
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    map: IndexMap<Column, Type>,
}

#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Row {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut entries = self.map.clone();
        entries.sort_keys();
        entries.keys().collect::<Vec<&Column>>().hash(state);
        entries.values().collect::<Vec<&Type>>().hash(state);
    }
}

impl PartialOrd for Row {
    /// Compare two dictionaries for sort ordering
    fn partial_cmp(&self, other: &Row) -> Option<Ordering> {
        let this: Vec<&Column> = self.map.keys().collect();
        let that: Vec<&Column> = other.map.keys().collect();

        if this != that {
            return this.partial_cmp(&that);
        }

        let this: Vec<&Type> = self.map.values().collect();
        let that: Vec<&Type> = self.map.values().collect();

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

        let this: Vec<&Type> = self.map.values().collect();
        let that: Vec<&Type> = self.map.values().collect();

        this.cmp(&that)
    }
}

impl Type {
    /// Convert a Primitive into its corresponding Type
    pub fn from_primitive(primitive: &Primitive) -> Type {
        match primitive {
            Primitive::Nothing => Type::Nothing,
            Primitive::Int(_) => Type::Int,
            Primitive::Range(range) => {
                let (left_value, left_inclusion) = &range.from;
                let (right_value, right_inclusion) = &range.to;

                let left_type = (Type::from_primitive(left_value), *left_inclusion);
                let right_type = (Type::from_primitive(right_value), *right_inclusion);

                let range = RangeType::new(left_type, right_type);
                Type::Range(Box::new(range))
            }
            Primitive::Decimal(_) => Type::Decimal,
            Primitive::Filesize(_) => Type::Filesize,
            Primitive::String(_) => Type::String,
            Primitive::ColumnPath(_) => Type::ColumnPath,
            Primitive::GlobPattern(_) => Type::GlobPattern,
            Primitive::Boolean(_) => Type::Boolean,
            Primitive::Date(_) => Type::Date,
            Primitive::Duration(_) => Type::Duration,
            Primitive::FilePath(_) => Type::FilePath,
            Primitive::Binary(_) => Type::Binary,
            Primitive::BeginningOfStream => Type::BeginningOfStream,
            Primitive::EndOfStream => Type::EndOfStream,
        }
    }

    /// Convert a dictionary into its corresponding row Type
    pub fn from_dictionary(dictionary: &Dictionary) -> Type {
        let mut map = IndexMap::new();

        for (key, value) in dictionary.entries.iter() {
            let column = Column::String(key.clone());
            map.insert(column, Type::from_value(value));
        }

        Type::Row(Row { map })
    }

    /// Convert a table into its corresponding Type
    pub fn from_table<'a>(table: impl IntoIterator<Item = &'a Value>) -> Type {
        let mut vec = vec![];

        for item in table.into_iter() {
            vec.push(Type::from_value(item))
        }

        Type::Table(vec)
    }

    /// Convert a value into its corresponding Type
    pub fn from_value<'a>(value: impl Into<&'a UntaggedValue>) -> Type {
        match value.into() {
            UntaggedValue::Primitive(p) => Type::from_primitive(p),
            UntaggedValue::Row(row) => Type::from_dictionary(row),
            UntaggedValue::Table(table) => Type::from_table(table.iter()),
            UntaggedValue::Error(_) => Type::Error,
            UntaggedValue::Block(_) => Type::Block,
        }
    }
}

impl PrettyDebug for Type {
    /// Prepare Type for pretty-printing
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            Type::Nothing => ty("nothing"),
            Type::Int => ty("integer"),
            Type::Range(range) => {
                let (left, left_inclusion) = &range.from;
                let (right, right_inclusion) = &range.to;

                let left_bracket = DbgDocBldr::delimiter(match left_inclusion {
                    RangeInclusion::Exclusive => "(",
                    RangeInclusion::Inclusive => "[",
                });

                let right_bracket = DbgDocBldr::delimiter(match right_inclusion {
                    RangeInclusion::Exclusive => ")",
                    RangeInclusion::Inclusive => "]",
                });

                DbgDocBldr::typed(
                    "range",
                    (left_bracket
                        + left.pretty()
                        + DbgDocBldr::operator(",")
                        + DbgDocBldr::space()
                        + right.pretty()
                        + right_bracket)
                        .group(),
                )
            }
            Type::Decimal => ty("decimal"),
            Type::Filesize => ty("filesize"),
            Type::String => ty("string"),
            Type::Line => ty("line"),
            Type::ColumnPath => ty("column-path"),
            Type::GlobPattern => ty("pattern"),
            Type::Boolean => ty("boolean"),
            Type::Date => ty("date"),
            Type::Duration => ty("duration"),
            Type::FilePath => ty("path"),
            Type::Binary => ty("binary"),
            Type::Error => DbgDocBldr::error("error"),
            Type::BeginningOfStream => DbgDocBldr::keyword("beginning-of-stream"),
            Type::EndOfStream => DbgDocBldr::keyword("end-of-stream"),
            Type::Row(row) => (DbgDocBldr::kind("row")
                + DbgDocBldr::space()
                + DbgDocBldr::intersperse(
                    row.map.iter().map(|(key, ty)| {
                        (DbgDocBldr::key(match key {
                            Column::String(string) => string.clone(),
                            Column::Value => "".to_string(),
                        }) + DbgDocBldr::delimit("(", ty.pretty(), ")").into_kind())
                        .nest()
                    }),
                    DbgDocBldr::space(),
                )
                .nest())
            .nest(),

            Type::Table(table) => {
                let mut group: Group<DebugDoc, Vec<(usize, usize)>> = Group::new();

                for (i, item) in table.iter().enumerate() {
                    group.add(item.to_doc(), i);
                }

                (DbgDocBldr::kind("table") + DbgDocBldr::space() + DbgDocBldr::keyword("of"))
                    .group()
                    + DbgDocBldr::space()
                    + (if group.len() == 1 {
                        let (doc, _) = group.into_iter().collect::<Vec<_>>()[0].clone();
                        DebugDocBuilder::from_doc(doc)
                    } else {
                        DbgDocBldr::intersperse(
                            group.into_iter().map(|(doc, rows)| {
                                (DbgDocBldr::intersperse(
                                    rows.iter().map(|(from, to)| {
                                        if from == to {
                                            DbgDocBldr::description(from)
                                        } else {
                                            (DbgDocBldr::description(from)
                                                + DbgDocBldr::space()
                                                + DbgDocBldr::keyword("to")
                                                + DbgDocBldr::space()
                                                + DbgDocBldr::description(to))
                                            .group()
                                        }
                                    }),
                                    DbgDocBldr::description(", "),
                                ) + DbgDocBldr::description(":")
                                    + DbgDocBldr::space()
                                    + DebugDocBuilder::from_doc(doc))
                                .nest()
                            }),
                            DbgDocBldr::space(),
                        )
                    })
            }
            Type::Block => ty("block"),
        }
    }
}

/// A view into dictionaries for debug purposes
#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a Column,
    value: &'a Type,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    /// Prepare debug entries for pretty-printing
    fn pretty(&self) -> DebugDocBuilder {
        DbgDocBldr::key(match self.key {
            Column::String(string) => string.clone(),
            Column::Value => "".to_string(),
        }) + DbgDocBldr::delimit("(", self.value.pretty(), ")").into_kind()
    }
}

/// Helper to create a pretty-print for the type
fn ty(name: impl std::fmt::Display) -> DebugDocBuilder {
    DbgDocBldr::kind(format!("{}", name))
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

#[derive(Debug)]
pub struct Group<K: Debug + Eq + Hash, V: GroupedValue> {
    values: indexmap::IndexMap<K, V>,
}

impl<K, G> Group<K, G>
where
    K: Debug + Eq + Hash,
    G: GroupedValue,
{
    pub fn new() -> Group<K, G> {
        Group {
            values: indexmap::IndexMap::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn into_iter(self) -> impl Iterator<Item = (K, G)> {
        self.values.into_iter()
    }

    pub fn add(&mut self, key: impl Into<K>, value: impl Into<G::Item>) {
        let key = key.into();
        let value = value.into();

        let group = self.values.get_mut(&key);

        match group {
            None => {
                self.values.insert(key, {
                    let mut group = G::new();
                    group.merge(value);
                    group
                });
            }
            Some(group) => {
                group.merge(value);
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Hash)]
pub enum Column {
    String(String),
    Value,
}

impl From<String> for Column {
    fn from(x: String) -> Self {
        Column::String(x)
    }
}

impl From<&String> for Column {
    fn from(x: &String) -> Self {
        Column::String(x.clone())
    }
}

impl From<&str> for Column {
    fn from(x: &str) -> Self {
        Column::String(x.to_string())
    }
}
