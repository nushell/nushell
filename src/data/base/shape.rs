use crate::data::base::{Block, ColumnPath};
use crate::data::dict::Dictionary;
use crate::prelude::*;
use crate::traits::{DebugDocBuilder as b, PrettyDebug};
use chrono::{DateTime, Utc};
use chrono_humanize::Humanize;
use derive_new::new;
use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::io::Write;
use std::path::PathBuf;

/**
  This file describes the structural types of the nushell system.

  Its primary purpose today is to identify "equivalent" values for the purpose
  of merging rows into a single table or identify rows in a table that have the
  same shape for reflection.

  It also serves as the primary vehicle for pretty-printing.
*/

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TypeShape {
    Nothing,
    Int,
    Decimal,
    Bytesize,
    String,
    ColumnPath,
    Pattern,
    Boolean,
    Date,
    Duration,
    Path,
    Binary,

    Row(BTreeMap<Column, TypeShape>),
    Table(Vec<TypeShape>),

    // TODO: Block arguments
    Block,
    // TODO: Error type
    Error,

    // Stream markers (used as bookend markers rather than actual values)
    BeginningOfStream,
    EndOfStream,
}

impl TypeShape {
    pub fn from_primitive(primitive: &Primitive) -> TypeShape {
        match primitive {
            Primitive::Nothing => TypeShape::Nothing,
            Primitive::Int(_) => TypeShape::Int,
            Primitive::Decimal(_) => TypeShape::Decimal,
            Primitive::Bytes(_) => TypeShape::Bytesize,
            Primitive::String(_) => TypeShape::String,
            Primitive::ColumnPath(_) => TypeShape::ColumnPath,
            Primitive::Pattern(_) => TypeShape::Pattern,
            Primitive::Boolean(_) => TypeShape::Boolean,
            Primitive::Date(_) => TypeShape::Date,
            Primitive::Duration(_) => TypeShape::Duration,
            Primitive::Path(_) => TypeShape::Path,
            Primitive::Binary(_) => TypeShape::Binary,
            Primitive::BeginningOfStream => TypeShape::BeginningOfStream,
            Primitive::EndOfStream => TypeShape::EndOfStream,
        }
    }

    pub fn from_dictionary(dictionary: &Dictionary) -> TypeShape {
        let mut map = BTreeMap::new();

        for (key, value) in dictionary.entries.iter() {
            let column = Column::String(key.clone());
            map.insert(column, TypeShape::from_value(&value.item));
        }

        TypeShape::Row(map)
    }

    pub fn from_table<'a>(table: impl IntoIterator<Item = &'a Value>) -> TypeShape {
        let mut vec = vec![];

        for item in table.into_iter() {
            vec.push(TypeShape::from_value(item))
        }

        TypeShape::Table(vec)
    }

    pub fn from_value(value: &Value) -> TypeShape {
        match value {
            Value::Primitive(p) => TypeShape::from_primitive(p),
            Value::Row(row) => TypeShape::from_dictionary(row),
            Value::Table(table) => TypeShape::from_table(table.iter().map(|i| &i.item)),
            Value::Error(_) => TypeShape::Error,
            Value::Block(_) => TypeShape::Block,
        }
    }
}

impl PrettyDebug for TypeShape {
    fn pretty_debug(&self) -> DebugDocBuilder {
        match self {
            TypeShape::Nothing => ty("nothing"),
            TypeShape::Int => ty("integer"),
            TypeShape::Decimal => ty("decimal"),
            TypeShape::Bytesize => ty("bytesize"),
            TypeShape::String => ty("string"),
            TypeShape::ColumnPath => ty("column-path"),
            TypeShape::Pattern => ty("pattern"),
            TypeShape::Boolean => ty("boolean"),
            TypeShape::Date => ty("date"),
            TypeShape::Duration => ty("duration"),
            TypeShape::Path => ty("path"),
            TypeShape::Binary => ty("binary"),
            TypeShape::Error => b::error("error"),
            TypeShape::BeginningOfStream => b::keyword("beginning-of-stream"),
            TypeShape::EndOfStream => b::keyword("end-of-stream"),
            TypeShape::Row(row) => (b::kind("row")
                + b::space()
                + b::intersperse(
                    row.iter().map(|(key, ty)| {
                        (b::key(match key {
                            Column::String(string) => string.clone(),
                            Column::Value => "<value>".to_string(),
                        }) + b::delimit("(", ty.pretty_debug(), ")").as_kind())
                        .nest()
                    }),
                    b::space(),
                )
                .nest())
            .nest(),

            TypeShape::Table(table) => {
                let mut group: Group<DebugDoc, Vec<(usize, usize)>> = Group::new();

                for (i, item) in table.iter().enumerate() {
                    group.add(item.to_doc(), i);
                }

                (b::kind("table") + b::space() + b::keyword("of")).group()
                    + b::space()
                    + (if group.len() == 1 {
                        let (doc, _) = group.into_iter().nth(0).unwrap();
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
            TypeShape::Block => ty("block"),
        }
    }
}

#[derive(Debug, new)]
struct DebugEntry<'a> {
    key: &'a Column,
    value: &'a TypeShape,
}

impl<'a> PrettyDebug for DebugEntry<'a> {
    fn pretty_debug(&self) -> DebugDocBuilder {
        (b::key(match self.key {
            Column::String(string) => string.clone(),
            Column::Value => format!("<value>"),
        }) + b::delimit("(", self.value.pretty_debug(), ")").as_kind())
    }
}

fn ty(name: impl std::fmt::Display) -> DebugDocBuilder {
    b::kind(format!("{}", name))
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum InlineShape {
    Nothing,
    Int(BigInt),
    Decimal(BigDecimal),
    Bytesize(u64),
    String(String),
    ColumnPath(ColumnPath),
    Pattern(String),
    Boolean(bool),
    Date(DateTime<Utc>),
    Duration(u64),
    Path(PathBuf),
    Binary,

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
            Primitive::Decimal(decimal) => InlineShape::Decimal(decimal.clone()),
            Primitive::Bytes(bytesize) => InlineShape::Bytesize(*bytesize),
            Primitive::String(string) => InlineShape::String(string.clone()),
            Primitive::ColumnPath(path) => InlineShape::ColumnPath(path.clone()),
            Primitive::Pattern(pattern) => InlineShape::Pattern(pattern.clone()),
            Primitive::Boolean(boolean) => InlineShape::Boolean(*boolean),
            Primitive::Date(date) => InlineShape::Date(date.clone()),
            Primitive::Duration(duration) => InlineShape::Duration(*duration),
            Primitive::Path(path) => InlineShape::Path(path.clone()),
            Primitive::Binary(_) => InlineShape::Binary,
            Primitive::BeginningOfStream => InlineShape::BeginningOfStream,
            Primitive::EndOfStream => InlineShape::EndOfStream,
        }
    }

    pub fn from_dictionary(dictionary: &Dictionary) -> InlineShape {
        let mut map = BTreeMap::new();

        for (key, value) in dictionary.entries.iter() {
            let column = Column::String(key.clone());
            map.insert(column, InlineShape::from_value(&value.item));
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

    pub fn from_value(value: &Value) -> InlineShape {
        match value {
            Value::Primitive(p) => InlineShape::from_primitive(p),
            Value::Row(row) => InlineShape::from_dictionary(row),
            Value::Table(table) => InlineShape::from_table(table.iter().map(|i| &i.item)),
            Value::Error(_) => InlineShape::Error,
            Value::Block(_) => InlineShape::Block,
        }
    }

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
    fn pretty_debug(&self) -> DebugDocBuilder {
        let column = &self.column;

        match &self.shape {
            InlineShape::Nothing => b::blank(),
            InlineShape::Int(int) => b::primitive(format!("{}", int)),
            InlineShape::Decimal(decimal) => b::primitive(format!("{}", decimal)),
            InlineShape::Bytesize(bytesize) => {
                let byte = byte_unit::Byte::from_bytes(*bytesize as u128);

                if byte.get_bytes() == 0u128 {
                    return b::description("—".to_string());
                }

                let byte = byte.get_appropriate_unit(false);

                match byte.get_unit() {
                    byte_unit::ByteUnit::B => {
                        (b::primitive(format!("{}", byte.get_value())) + b::space() + b::kind("B"))
                            .group()
                    }
                    _ => b::primitive(format!("{}", byte.format(1))),
                }
            }
            InlineShape::String(string) => b::primitive(format!("{}", string)),
            InlineShape::ColumnPath(path) => b::intersperse(
                path.iter().map(|member| member.pretty_debug()),
                b::keyword("."),
            ),
            InlineShape::Pattern(pattern) => b::primitive(pattern),
            InlineShape::Boolean(boolean) => b::primitive(match (boolean, column) {
                (true, None) => format!("Yes"),
                (false, None) => format!("No"),
                (true, Some(Column::String(s))) if !s.is_empty() => format!("{}", s),
                (false, Some(Column::String(s))) if !s.is_empty() => format!(""),
                (true, Some(_)) => format!("Yes"),
                (false, Some(_)) => format!("No"),
            }),
            InlineShape::Date(date) => b::primitive(date.humanize()),
            InlineShape::Duration(duration) => {
                (b::kind("duration") + b::space() + b::primitive(duration)).group()
            }
            InlineShape::Path(path) => b::primitive(path.display()),
            InlineShape::Binary => b::opaque("<binary>"),
            InlineShape::Row(row) => b::delimit(
                "[",
                b::kind("row")
                    + b::space()
                    + b::intersperse(
                        row.keys().map(|key| match key {
                            Column::String(string) => b::description(string),
                            Column::Value => b::blank(),
                        }),
                        b::space(),
                    ),
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
                    group.merge(value.into());
                    group
                });
            }
            Some(group) => {
                group.merge(value.into());
            }
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Shape {
    Primitive(&'static str),
    Row(Vec<Column>),
    Table { from: usize, to: usize },
    Error(ShellError),
    Block(Block),
}

impl Value {
    pub fn shape(&self) -> Shape {
        Shape::for_value(self)
    }
}

impl Shape {
    pub fn for_value(value: &Value) -> Shape {
        match value {
            Value::Primitive(p) => Shape::Primitive(p.type_name()),
            Value::Row(row) => Shape::for_dict(row),
            Value::Table(table) => Shape::Table {
                from: 0,
                to: table.len(),
            },
            Value::Error(error) => Shape::Error(error.clone()),
            Value::Block(block) => Shape::Block(block.clone()),
        }
    }

    fn for_dict(dict: &Dictionary) -> Shape {
        Shape::Row(dict.keys().map(|key| Column::String(key.clone())).collect())
    }

    pub fn kind(&self) -> String {
        match self {
            Shape::Primitive(primitive) => primitive,
            Shape::Row(row) => {
                return row
                    .iter()
                    .map(|c| match c {
                        Column::String(s) => s.clone(),
                        Column::Value => format!("<value>"),
                    })
                    .join(", ")
            }
            Shape::Table { .. } => "table",
            Shape::Error(_) => "error",
            Shape::Block(_) => "block",
        }
        .to_string()
    }

    pub fn describe_str(&self) -> String {
        let mut v = vec![];
        self.describe(&mut v)
            .expect("it isn't possible to fail to write into a memory buffer");
        String::from_utf8_lossy(&v[..]).to_string()
    }

    pub fn describe(&self, w: &mut impl Write) -> Result<(), std::io::Error> {
        match self {
            Shape::Primitive(desc) => write!(w, "[{}]", desc),
            Shape::Row(d) => write!(
                w,
                "[row: {}]",
                d.iter()
                    .map(|c| match c {
                        Column::String(s) => s.clone(),
                        Column::Value => format!("<value>"),
                    })
                    .join(", ")
            ),
            Shape::Table { to, .. } => {
                if *to == 1 {
                    write!(w, "[table: {} row]", to)
                } else {
                    write!(w, "[table: {} rows]", to)
                }
            }
            Shape::Error(_) => write!(w, "[error]"),
            Shape::Block(_) => write!(w, "[block]"),
        }
    }

    fn to_value(&self) -> Value {
        let mut out = vec![];
        self.describe(&mut out)
            .expect("Writing into a Vec can't fail");
        let string = String::from_utf8_lossy(&out);

        Value::string(string)
    }
}

pub struct Shapes {
    shapes: IndexMap<Shape, Vec<usize>>,
}

impl Shapes {
    pub fn new() -> Shapes {
        Shapes {
            shapes: IndexMap::default(),
        }
    }

    pub fn add(&mut self, value: &Value, row: usize) {
        let shape = Shape::for_value(value);

        self.shapes
            .entry(shape)
            .and_modify(|indexes| indexes.push(row))
            .or_insert_with(|| vec![row]);
    }

    pub fn to_values(&self) -> Vec<Tagged<Value>> {
        if self.shapes.len() == 1 {
            let shape = self.shapes.keys().nth(0).unwrap();

            vec![dict! {
                "type" => shape.to_value(),
                "rows" => Value::string("all")
            }]
        } else {
            self.shapes
                .iter()
                .map(|(shape, rows)| {
                    let rows = rows.iter().map(|i| i.to_string()).join(", ");

                    dict! {
                        "type" => shape.to_value(),
                        "rows" => Value::string(format!("[ {} ]", rows))
                    }
                })
                .collect()
        }
    }
}
