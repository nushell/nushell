use crate::commands::WholeStreamCommand;
use crate::prelude::*;
use hex::encode;
use nu_errors::ShellError;
use nu_protocol::{Dictionary, Primitive, ReturnSuccess, Signature, UntaggedValue, Value};
use rusqlite::{Connection, NO_PARAMS};
use std::io::Read;

pub struct ToSQLite;

impl WholeStreamCommand for ToSQLite {
    fn name(&self) -> &str {
        "to sqlite"
    }

    fn signature(&self) -> Signature {
        Signature::build("to sqlite")
    }

    fn usage(&self) -> &str {
        "Convert table to sqlite .db binary data"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_sqlite(args, registry)
    }

    fn is_binary(&self) -> bool {
        true
    }
}

pub struct ToDB;

impl WholeStreamCommand for ToDB {
    fn name(&self) -> &str {
        "to db"
    }

    fn signature(&self) -> Signature {
        Signature::build("to db")
    }

    fn usage(&self) -> &str {
        "Convert table to db data"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        to_sqlite(args, registry)
    }

    fn is_binary(&self) -> bool {
        true
    }
}

fn comma_concat(acc: String, current: String) -> String {
    if acc == "" {
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
            Primitive::Int(i) => format!("{}", i),
            Primitive::Duration(u) => format!("{}", u),
            Primitive::Decimal(f) => format!("{}", f),
            Primitive::Bytes(u) => format!("{}", u),
            Primitive::Pattern(s) => format!("'{}'", s.replace("'", "''")),
            Primitive::String(s) => format!("'{}'", s.replace("'", "''")),
            Primitive::Line(s) => format!("'{}'", s.replace("'", "''")),
            Primitive::Boolean(true) => "1".into(),
            Primitive::Boolean(_) => "0".into(),
            Primitive::Date(d) => format!("'{}'", d),
            Primitive::Path(p) => format!("'{}'", p.display().to_string().replace("'", "''")),
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
    Ok(values.into_iter().fold("".to_string(), comma_concat))
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
        }) => (get_columns(l), get_insert_values(l.to_vec())),
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
    for value in values.into_iter() {
        match &value.value {
            UntaggedValue::Row(d) => {
                let (create, insert) = generate_statements(d.to_owned())?;
                match conn
                    .execute(&create, NO_PARAMS)
                    .and_then(|_| conn.execute(&insert, NO_PARAMS))
                {
                    Ok(_) => (),
                    Err(e) => {
                        outln!("{}", create);
                        outln!("{}", insert);
                        outln!("{:?}", e);
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

fn to_sqlite(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;
        let name_tag = args.name_tag();
        let input: Vec<Value> = args.input.collect().await;

        match sqlite_input_stream_to_bytes(input) {
            Ok(out) => yield ReturnSuccess::value(out),
            _ => {
                yield Err(ShellError::labeled_error(
                    "Expected a table with SQLite-compatible structure from pipeline",
                    "requires SQLite-compatible input",
                    name_tag,
                ))
            },
        }
    };

    Ok(stream.to_output_stream())
}

#[cfg(test)]
mod tests {
    use super::ToSQLite;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(ToSQLite {})
    }
}
