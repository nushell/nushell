use bigdecimal::FromPrimitive;
use nu_errors::ShellError;
use nu_protocol::{Primitive, ReturnSuccess, ReturnValue, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use rusqlite::{types::ValueRef, Connection, Row};
use std::io::Write;
use std::path::Path;

#[derive(Default)]
pub struct FromSqlite {
    pub state: Vec<u8>,
    pub name_tag: Tag,
    pub tables: Vec<String>,
}

impl FromSqlite {
    pub fn new() -> FromSqlite {
        FromSqlite {
            state: vec![],
            name_tag: Tag::unknown(),
            tables: vec![],
        }
    }
}

pub fn convert_sqlite_file_to_nu_value(
    path: &Path,
    tag: impl Into<Tag> + Clone,
    tables: Vec<String>,
) -> Result<Value, rusqlite::Error> {
    let conn = Connection::open(path)?;

    let mut meta_out = Vec::new();
    let mut meta_stmt = conn.prepare("select name from sqlite_master where type='table'")?;
    let mut meta_rows = meta_stmt.query([])?;

    while let Some(meta_row) = meta_rows.next()? {
        let table_name: String = meta_row.get(0)?;
        if tables.is_empty() || tables.contains(&table_name) {
            let mut meta_dict = TaggedDictBuilder::new(tag.clone());
            let mut out = Vec::new();
            let mut table_stmt = conn.prepare(&format!("select * from [{}]", table_name))?;
            let mut table_rows = table_stmt.query([])?;
            while let Some(table_row) = table_rows.next()? {
                out.push(convert_sqlite_row_to_nu_value(table_row, tag.clone()))
            }
            meta_dict.insert_value(
                "table_name".to_string(),
                UntaggedValue::Primitive(Primitive::String(table_name)).into_value(tag.clone()),
            );
            meta_dict.insert_value(
                "table_values",
                UntaggedValue::Table(out).into_value(tag.clone()),
            );
            meta_out.push(meta_dict.into_value());
        }
    }
    let tag = tag.into();
    Ok(UntaggedValue::Table(meta_out).into_value(tag))
}

fn convert_sqlite_row_to_nu_value(row: &Row, tag: impl Into<Tag> + Clone) -> Value {
    let mut collected = TaggedDictBuilder::new(tag.clone());
    for (i, c) in row.as_ref().column_names().iter().enumerate() {
        collected.insert_value(
            c.to_string(),
            convert_sqlite_value_to_nu_value(row.get_ref_unwrap(i), tag.clone()),
        );
    }
    collected.into_value()
}

fn convert_sqlite_value_to_nu_value(value: ValueRef, tag: impl Into<Tag> + Clone) -> Value {
    match value {
        ValueRef::Null => {
            UntaggedValue::Primitive(Primitive::String(String::from(""))).into_value(tag)
        }
        ValueRef::Integer(i) => UntaggedValue::int(i).into_value(tag),
        ValueRef::Real(f) => {
            let f = bigdecimal::BigDecimal::from_f64(f);
            let tag = tag.into();
            let span = tag.span;
            match f {
                Some(d) => UntaggedValue::decimal(d).into_value(tag),
                None => UntaggedValue::Error(ShellError::labeled_error(
                    "Can not convert f64 to big decimal",
                    "can not convert to decimal",
                    span,
                ))
                .into_value(tag),
            }
        }
        ValueRef::Text(s) => {
            // this unwrap is safe because we know the ValueRef is Text.
            UntaggedValue::Primitive(Primitive::String(String::from_utf8_lossy(s).to_string()))
                .into_value(tag)
        }
        ValueRef::Blob(u) => UntaggedValue::binary(u.to_owned()).into_value(tag),
    }
}

pub fn from_sqlite_bytes_to_value(
    mut bytes: Vec<u8>,
    tag: impl Into<Tag> + Clone,
    tables: Vec<String>,
) -> Result<Value, std::io::Error> {
    // FIXME: should probably write a sqlite virtual filesystem
    // that will allow us to use bytes as a file to avoid this
    // write out, but this will require C code. Might be
    // best done as a PR to rusqlite.
    let mut tempfile = tempfile::NamedTempFile::new()?;
    tempfile.write_all(bytes.as_mut_slice())?;
    match convert_sqlite_file_to_nu_value(tempfile.path(), tag, tables) {
        Ok(value) => Ok(value),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

pub fn from_sqlite(
    bytes: Vec<u8>,
    name_tag: Tag,
    tables: Vec<String>,
) -> Result<Vec<ReturnValue>, ShellError> {
    match from_sqlite_bytes_to_value(bytes, name_tag.clone(), tables) {
        Ok(x) => match x {
            Value {
                value: UntaggedValue::Table(list),
                ..
            } => Ok(list.into_iter().map(ReturnSuccess::value).collect()),
            _ => Ok(vec![ReturnSuccess::value(x)]),
        },
        Err(_) => Err(ShellError::labeled_error(
            "Could not parse as SQLite",
            "input cannot be parsed as SQLite",
            &name_tag,
        )),
    }
}
