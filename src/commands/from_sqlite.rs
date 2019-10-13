use crate::commands::WholeStreamCommand;
use crate::data::{Primitive, TaggedDictBuilder, Value};
use crate::errors::ShellError;
use crate::prelude::*;
use rusqlite::{types::ValueRef, Connection, Row, NO_PARAMS};
use std::io::Write;
use std::path::Path;

pub struct FromSQLite;

impl WholeStreamCommand for FromSQLite {
    fn name(&self) -> &str {
        "from-sqlite"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-sqlite")
    }

    fn usage(&self) -> &str {
        "Parse binary data as sqlite .db and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_sqlite(args, registry)
    }
}

pub struct FromDB;

impl WholeStreamCommand for FromDB {
    fn name(&self) -> &str {
        "from-db"
    }

    fn signature(&self) -> Signature {
        Signature::build("from-db")
    }

    fn usage(&self) -> &str {
        "Parse binary data as db and create table."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        from_sqlite(args, registry)
    }
}

pub fn convert_sqlite_file_to_nu_value(
    path: &Path,
    tag: impl Into<Tag> + Clone,
) -> Result<Tagged<Value>, rusqlite::Error> {
    let conn = Connection::open(path)?;

    let mut meta_out = Vec::new();
    let mut meta_stmt = conn.prepare("select name from sqlite_master where type='table'")?;
    let mut meta_rows = meta_stmt.query(NO_PARAMS)?;
    while let Some(meta_row) = meta_rows.next()? {
        let table_name: String = meta_row.get(0)?;
        let mut meta_dict = TaggedDictBuilder::new(tag.clone());
        let mut out = Vec::new();
        let mut table_stmt = conn.prepare(&format!("select * from [{}]", table_name))?;
        let mut table_rows = table_stmt.query(NO_PARAMS)?;
        while let Some(table_row) = table_rows.next()? {
            out.push(convert_sqlite_row_to_nu_value(table_row, tag.clone())?)
        }
        meta_dict.insert_tagged(
            "table_name".to_string(),
            Value::Primitive(Primitive::String(table_name)).tagged(tag.clone()),
        );
        meta_dict.insert_tagged("table_values", Value::Table(out).tagged(tag.clone()));
        meta_out.push(meta_dict.into_tagged_value());
    }
    let tag = tag.into();
    Ok(Value::Table(meta_out).tagged(tag))
}

fn convert_sqlite_row_to_nu_value(
    row: &Row,
    tag: impl Into<Tag> + Clone,
) -> Result<Tagged<Value>, rusqlite::Error> {
    let mut collected = TaggedDictBuilder::new(tag.clone());
    for (i, c) in row.columns().iter().enumerate() {
        collected.insert_tagged(
            c.name().to_string(),
            convert_sqlite_value_to_nu_value(row.get_raw(i), tag.clone()),
        );
    }
    return Ok(collected.into_tagged_value());
}

fn convert_sqlite_value_to_nu_value(value: ValueRef, tag: impl Into<Tag> + Clone) -> Tagged<Value> {
    match value {
        ValueRef::Null => Value::Primitive(Primitive::String(String::from(""))).tagged(tag),
        ValueRef::Integer(i) => Value::number(i).tagged(tag),
        ValueRef::Real(f) => Value::number(f).tagged(tag),
        t @ ValueRef::Text(_) => {
            // this unwrap is safe because we know the ValueRef is Text.
            Value::Primitive(Primitive::String(t.as_str().unwrap().to_string())).tagged(tag)
        }
        ValueRef::Blob(u) => Value::binary(u.to_owned()).tagged(tag),
    }
}

pub fn from_sqlite_bytes_to_value(
    mut bytes: Vec<u8>,
    tag: impl Into<Tag> + Clone,
) -> Result<Tagged<Value>, std::io::Error> {
    // FIXME: should probably write a sqlite virtual filesystem
    // that will allow us to use bytes as a file to avoid this
    // write out, but this will require C code. Might be
    // best done as a PR to rusqlite.
    let mut tempfile = tempfile::NamedTempFile::new()?;
    tempfile.write_all(bytes.as_mut_slice())?;
    match convert_sqlite_file_to_nu_value(tempfile.path(), tag) {
        Ok(value) => Ok(value),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
}

fn from_sqlite(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let tag = args.name_tag();
    let input = args.input;

    let stream = async_stream! {
        let values: Vec<Tagged<Value>> = input.values.collect().await;

        for value in values {
            let value_tag = value.tag();
            match value.item {
                Value::Primitive(Primitive::Binary(vb)) =>
                    match from_sqlite_bytes_to_value(vb, tag.clone()) {
                        Ok(x) => match x {
                            Tagged { item: Value::Table(list), .. } => {
                                for l in list {
                                    yield ReturnSuccess::value(l);
                                }
                            }
                            _ => yield ReturnSuccess::value(x),
                        }
                        Err(_) => {
                            yield Err(ShellError::labeled_error_with_secondary(
                                "Could not parse as SQLite",
                                "input cannot be parsed as SQLite",
                                &tag,
                                "value originates from here",
                                value_tag,
                            ))
                        }
                    }
                _ => yield Err(ShellError::labeled_error_with_secondary(
                    "Expected a string from pipeline",
                    "requires string input",
                    &tag,
                    "value originates from here",
                    value_tag,
                )),

            }
        }
    };

    Ok(stream.to_output_stream())
}
