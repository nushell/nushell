use crate::value::dict::Dictionary;
use crate::value::primitive::Primitive;
use crate::value::range::RangeInclusion;
use crate::value::{UntaggedValue, Value};
use derive_new::new;
use nu_source::{b, DebugDoc, DebugDocBuilder, PrettyDebug};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

/**
  This file describes the structural types of the nushell system.

  Its primary purpose today is to identify "equivalent" values for the purpose
  of merging rows into a single table or identify rows in a table that have the
  same shape for reflection.
*/

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, new)]
pub struct RangeType {
    from: (Type, RangeInclusion),
    to: (Type, RangeInclusion),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Type {
    Nothing,
    Int,
    Range(Box<RangeType>),
    Decimal,
    Bytesize,
    String,
    Line,
    ColumnPath,
    Pattern,
    Boolean,
    Date,
    Duration,
    Path,
    Binary,

    Row(Row),
    Table(Vec<Type>),

    // TODO: Block arguments
    Block,
    // TODO: Error type
    Error,

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, new)]
pub struct Row {
    #[new(default)]
    map: BTreeMap<Column, Type>,
}

impl Serialize for Row {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_map(self.map.iter())
    }
}

impl<'de> Deserialize<'de> for Row {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RowVisitor;

        impl<'de> serde::de::Visitor<'de> for RowVisitor {
            type Value = Row;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a row")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut new_map = BTreeMap::new();

                loop {
                    let entry = map.next_entry()?;

                    match entry {
                        None => return Ok(Row { map: new_map }),
                        Some((key, value)) => {
                            new_map.insert(key, value);
                        }
                    }
                }
            }
        }
        deserializer.deserialize_map(RowVisitor)
    }
}

impl Type {
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
            Primitive::Bytes(_) => Type::Bytesize,
            Primitive::String(_) => Type::String,
            Primitive::Line(_) => Type::Line,
            Primitive::ColumnPath(_) => Type::ColumnPath,
            Primitive::Pattern(_) => Type::Pattern,
            Primitive::Boolean(_) => Type::Boolean,
            Primitive::Date(_) => Type::Date,
            Primitive::Duration(_) => Type::Duration,
            Primitive::Path(_) => Type::Path,
            Primitive::Binary(_) => Type::Binary,
            Primitive::BeginningOfStream => Type::BeginningOfStream,
            Primitive::EndOfStream => Type::EndOfStream,
        }
    }

    pub fn from_dictionary(dictionary: &Dictionary) -> Type {
        let mut map = BTreeMap::new();

        for (key, value) in dictionary.entries.iter() {
            let column = Column::String(key.clone());
            map.insert(column, Type::from_value(value));
        }

        Type::Row(Row { map })
    }

    pub fn from_table<'a>(table: impl IntoIterator<Item = &'a Value>) -> Type {
        let mut vec = vec![];

        for item in table.into_iter() {
            vec.push(Type::from_value(item))
        }

        Type::Table(vec)
    }

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
    fn pretty(&self) -> DebugDocBuilder {
        match self {
            Type::Nothing => ty("nothing"),
            Type::Int => ty("integer"),
            Type::Range(range) => {
                let (left, left_inclusion) = &range.from;
                let (right, right_inclusion) = &range.to;

                let left_bracket = b::delimiter(match left_inclusion {
                    RangeInclusion::Exclusive => "(",
                    RangeInclusion::Inclusive => "[",
                });

                let right_bracket = b::delimiter(match right_inclusion {
                    RangeInclusion::Exclusive => ")",
                    RangeInclusion::Inclusive => "]",
                });

                b::typed(
                    "range",
                    (left_bracket
                        + left.pretty()
                        + b::operator(",")
                        + b::space()
                        + right.pretty()
                        + right_bracket)
                        .group(),
                )
            }
            Type::Decimal => ty("decimal"),
            Type::Bytesize => ty("bytesize"),
            Type::String => ty("string"),
            Type::Line => ty("line"),
            Type::ColumnPath => ty("column-path"),
            Type::Pattern => ty("pattern"),
            Type::Boolean => ty("boolean"),
            Type::Date => ty("date"),
            Type::Duration => ty("duration"),
            Type::Path => ty("path"),
            Type::Binary => ty("binary"),
            Type::Error => b::error("error"),
            Type::BeginningOfStream => b::keyword("beginning-of-stream"),
            Type::EndOfStream => b::keyword("end-of-stream"),
            Type::Row(row) => (b::kind("row")
                + b::space()
                + b::intersperse(
                    row.map.iter().map(|(key, ty)| {
                        (b::key(match key {
                            Column::String(string) => string.clone(),
                            Column::Value => "<value>".to_string(),
                        }) + b::delimit("(", ty.pretty(), ")").into_kind())
                        .nest()
                    }),
                    b::space(),
                )
                .nest())
            .nest(),

            Type::Table(table) => {
                let mut group: Group<DebugDoc, Vec<(usize, usize)>> = Group::new();

                for (i, item) in table.iter().enumerate() {
                    group.add(item.to_doc(), i);
                }

                (b::kind("table") + b::space() + b::keyword("of")).group()
                    + b::space()
                    + (if group.len() == 1 {
                        let (doc, _) = group.into_iter().collect::<Vec<_>>()[0].clone();
                        DebugDocBuilder::from_doc(doc)
                    } else {
                        b::intersperse(
                            group.into_iter().map(|(doc, rows)| {
                                (b::intersperse(
                                    rows.iter().map(|(from, to)| {
                                        if from == to {
                                            b::description(from)
                                        } else {
                                            (b::description(from)
                                                + b::space()
                                                + b::keyword("to")
                                                + b::space()
                                                + b::description(to))
                                            .group()
                                        }
                                    }),
                                    b::description(", "),
                                ) + b::description(":")
                                    + b::space()
                                    + DebugDocBuilder::from_doc(doc))
                                .nest()
                            }),
                            b::space(),
                        )
                    })
            }
            Type::Block => ty("block"),
        }
    }
}

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a Column,
    value: &'a Type,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty(&self) -> DebugDocBuilder {
        (b::key(match self.key {
            Column::String(string) => string.clone(),
            Column::Value => "<value>".to_string(),
        }) + b::delimit("(", self.value.pretty(), ")").into_kind())
    }
}

fn ty(name: impl std::fmt::Display) -> DebugDocBuilder {
    b::kind(format!("{}", name))
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
