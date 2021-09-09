use hex::encode;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, ReturnSuccess, ReturnValue, UntaggedValue, Value};
use nu_source::Tag;
use rusqlite::Connection;
use std::io::Read;

#[derive(Default)]
pub struct ToSqlite {
    pub state: Vec<Value>,
}

impl ToSqlite {
    pub fn new() -> ToSqlite {
        ToSqlite { state: vec![] }
    }
}
fn comma_concat(acc: String, current: String) -> String {
    if acc.is_empty() {
        current
    } else {
        format!("{}, {}", acc, current)
    }
}

fn get_columns(rows: &[Value]) -> Result<String, std::io::Error> {
    match &rows[0].value {
        UntaggedValue::Row(d) => Ok(d
            .entries
            .iter()
            .map(|(k, _v)| k.clone())
            .fold("".to_string(), comma_concat)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Could not find table column names",
        )),
    }
}

fn nu_value_to_sqlite_string(v: Value) -> String {
    match &v.value {
        UntaggedValue::Primitive(p) => match p {
            Primitive::Nothing => "NULL".into(),
            Primitive::BigInt(i) => i.to_string(),
            Primitive::Int(i) => i.to_string(),
            Primitive::Duration(i) => i.to_string(),
            Primitive::Decimal(f) => f.to_string(),
            Primitive::Filesize(u) => u.to_string(),
            Primitive::GlobPattern(s) => format!("'{}'", s.replace("'", "''")),
            Primitive::String(s) => format!("'{}'", s.replace("'", "''")),
            Primitive::Boolean(true) => "1".into(),
            Primitive::Boolean(_) => "0".into(),
            Primitive::Date(d) => format!("'{}'", d),
            Primitive::FilePath(p) => format!("'{}'", p.display().to_string().replace("'", "''")),
            Primitive::Binary(u) => format!("x'{}'", encode(u)),
            Primitive::BeginningOfStream
            | Primitive::EndOfStream
            | Primitive::ColumnPath(_)
            | Primitive::Range(_) => "NULL".into(),
        },
        _ => "NULL".into(),
    }
}

fn get_insert_values(rows: Vec<Value>) -> Result<String, std::io::Error> {
    let values: Result<Vec<_>, _> = rows
        .into_iter()
        .map(|value| match value.value {
            UntaggedValue::Row(d) => Ok(format!(
                "({})",
                d.entries
                    .iter()
                    .map(|(_k, v)| nu_value_to_sqlite_string(v.clone()))
                    .fold("".to_string(), comma_concat)
            )),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Could not find table column names",
            )),
        })
        .collect();
    let values = values?;
    Ok(values.join(", "))
}

fn generate_statements(table: Dictionary) -> Result<(String, String), std::io::Error> {
    let table_name = match table.entries.get("table_name") {
        Some(Value {
            value: UntaggedValue::Primitive(Primitive::String(table_name)),
            ..
        }) => table_name,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Could not find table name",
            ))
        }
    };
    let (columns, insert_values) = match table.entries.get("table_values") {
        Some(Value {
            value: UntaggedValue::Table(l),
            ..
        }) => {
            if l.is_empty() {
                return Ok((String::new(), String::new()));
            }
            (get_columns(l), get_insert_values(l.to_vec()))
        }
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Could not find table values",
            ))
        }
    };

    let create = format!("create table {}({})", table_name, columns?);
    let insert = format!("insert into {} values {}", table_name, insert_values?);
    Ok((create, insert))
}

fn sqlite_input_stream_to_bytes(values: Vec<Value>) -> Result<Value, std::io::Error> {
    // FIXME: should probably write a sqlite virtual filesystem
    // that will allow us to use bytes as a file to avoid this
    // write out, but this will require C code. Might be
    // best done as a PR to rusqlite.
    let mut tempfile = tempfile::NamedTempFile::new()?;
    let conn = match Connection::open(tempfile.path()) {
        Ok(conn) => conn,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    };
    let tag = values[0].tag.clone();
    for value in values {
        match &value.value {
            UntaggedValue::Row(d) => {
                let (create, insert) = generate_statements(d.to_owned())?;
                if create.is_empty() {
                    continue;
                }
                match conn
                    .execute(&create, [])
                    .and_then(|_| conn.execute(&insert, []))
                {
                    Ok(_) => (),
                    Err(e) => {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
                    }
                }
            }
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Expected row, found {:?}", other),
                ))
            }
        }
    }
    let mut out = Vec::new();
    tempfile.read_to_end(&mut out)?;
    Ok(UntaggedValue::binary(out).into_value(tag))
}

pub fn to_sqlite(input: Vec<Value>, name_tag: Tag) -> Result<Vec<ReturnValue>, ShellError> {
    match sqlite_input_stream_to_bytes(input) {
        Ok(out) => Ok(vec![ReturnSuccess::value(out)]),
        _ => Err(ShellError::labeled_error(
            "Expected a table with SQLite-compatible structure from pipeline",
            "requires SQLite-compatible input",
            name_tag,
        )),
    }
}
