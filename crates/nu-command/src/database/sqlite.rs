use std::path::{Path, PathBuf};

use nu_protocol::{CustomValue, ShellError, Span, Spanned, Value};
use rusqlite::{types::ValueRef, Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SQLiteDatabase {
    // I considered storing a SQLite connection here, but decided against it because
    // 1) YAGNI, 2) it's not obvious how cloning a connection could work, 3) state
    // management gets tricky quick. Revisit this approach if we find a compelling use case.
    path: PathBuf,
}

impl SQLiteDatabase {
    pub fn new(path: &Path) -> SQLiteDatabase {
        SQLiteDatabase {
            path: PathBuf::from(path),
        }
    }

    pub fn query(&self, sql: &Spanned<String>, call_span: Span) -> Result<Value, ShellError> {
        let db = open_sqlite_db(&self.path, call_span)?;
        to_shell_error(
            run_sql_query(db, sql),
            "Failed to query SQLite database",
            sql.span,
        )
    }

    pub fn describe(&self) -> String {
        format!("A SQLite database at {:?}", self.path)
    }
}

impl CustomValue for SQLiteDatabase {
    fn clone_value(&self, span: Span) -> Value {
        let cloned = SQLiteDatabase {
            path: self.path.clone(),
        };

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let db = open_sqlite_db(&self.path, span)?;
        to_shell_error(
            read_entire_sqlite_db(db, span),
            "Failed to read from SQLite database",
            span,
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(&self, _count: usize, span: Span) -> Result<Value, ShellError> {
        // In theory we could support this, but tables don't have an especially well-defined order
        Err(ShellError::IncompatiblePathAccess("SQLite databases do not support integer-indexed access. Try specifying a table name instead".into(), span))
    }

    fn follow_path_string(&self, _column_name: String, span: Span) -> Result<Value, ShellError> {
        let db = open_sqlite_db(&self.path, span)?;

        to_shell_error(
            read_single_table(db, _column_name, span),
            "Failed to read from SQLite database",
            span,
        )
    }

    fn typetag_name(&self) -> &'static str {
        "SQLiteDatabase"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

// TODO: is there a more elegant way to map rusqlite errors to ShellErrors?
fn to_shell_error<T>(
    result: Result<T, rusqlite::Error>,
    message: &str,
    span: Span,
) -> Result<T, nu_protocol::ShellError> {
    match result {
        Ok(val) => Ok(val),
        Err(err) => Err(ShellError::GenericError(
            message.to_string(),
            err.to_string(),
            Some(span),
            None,
            Vec::new(),
        )),
    }
}

fn open_sqlite_db(path: &Path, call_span: Span) -> Result<Connection, nu_protocol::ShellError> {
    let path = path.to_string_lossy().to_string();
    to_shell_error(
        Connection::open(path),
        "Failed to open SQLite database",
        call_span,
    )
}

fn read_entire_sqlite_db(conn: Connection, call_span: Span) -> Result<Value, rusqlite::Error> {
    let mut table_names: Vec<String> = Vec::new();
    let mut tables: Vec<Value> = Vec::new();

    let mut get_table_names =
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let rows = get_table_names.query_map([], |row| row.get(0))?;

    for row in rows {
        let table_name: String = row?;
        table_names.push(table_name.clone());

        let mut rows = Vec::new();
        let mut table_stmt = conn.prepare(&format!("select * from [{}]", table_name))?;
        let mut table_rows = table_stmt.query([])?;
        while let Some(table_row) = table_rows.next()? {
            rows.push(convert_sqlite_row_to_nu_value(table_row, call_span))
        }

        let table_record = Value::List {
            vals: rows,
            span: call_span,
        };

        tables.push(table_record);
    }

    Ok(Value::Record {
        cols: table_names,
        vals: tables,
        span: call_span,
    })
}

fn run_sql_query(conn: Connection, sql: &Spanned<String>) -> Result<Value, rusqlite::Error> {
    let mut stmt = conn.prepare(&sql.item)?;
    let mut results = stmt.query([])?;
    let mut nu_records = Vec::new();

    while let Some(table_row) = results.next()? {
        nu_records.push(convert_sqlite_row_to_nu_value(table_row, sql.span))
    }

    Ok(Value::List {
        vals: nu_records,
        span: sql.span,
    })
}

fn read_single_table(
    conn: Connection,
    table_name: String,
    call_span: Span,
) -> Result<Value, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM {}", table_name))?;
    let mut results = stmt.query([])?;
    let mut nu_records = Vec::new();

    while let Some(table_row) = results.next()? {
        nu_records.push(convert_sqlite_row_to_nu_value(table_row, call_span))
    }

    Ok(Value::List {
        vals: nu_records,
        span: call_span,
    })
}

fn convert_sqlite_row_to_nu_value(row: &Row, span: Span) -> Value {
    let mut vals = Vec::new();
    let colnamestr = row.as_ref().column_names().to_vec();
    let colnames = colnamestr.iter().map(|s| s.to_string()).collect();

    for (i, c) in row.as_ref().column_names().iter().enumerate() {
        let _column = c.to_string();
        let val = convert_sqlite_value_to_nu_value(row.get_ref_unwrap(i), span);
        vals.push(val);
    }

    Value::Record {
        cols: colnames,
        vals,
        span,
    }
}

fn convert_sqlite_value_to_nu_value(value: ValueRef, span: Span) -> Value {
    match value {
        ValueRef::Null => Value::Nothing { span },
        ValueRef::Integer(i) => Value::Int { val: i, span },
        ValueRef::Real(f) => Value::Float { val: f, span },
        ValueRef::Text(buf) => {
            let s = match std::str::from_utf8(buf) {
                Ok(v) => v,
                Err(_) => {
                    return Value::Error {
                        error: ShellError::NonUtf8(span),
                    }
                }
            };
            Value::String {
                val: s.to_string(),
                span,
            }
        }
        ValueRef::Blob(u) => Value::Binary {
            val: u.to_vec(),
            span,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_read_empty_db() {
        let db = Connection::open_in_memory().unwrap();
        let converted_db = read_entire_sqlite_db(db, Span::test_data()).unwrap();

        let expected = Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::test_data(),
        };

        assert_eq!(converted_db, expected);
    }

    #[test]
    fn can_read_empty_table() {
        let db = Connection::open_in_memory().unwrap();

        db.execute(
            "CREATE TABLE person (
                    id     INTEGER PRIMARY KEY,
                    name   TEXT NOT NULL,
                    data   BLOB
                    )",
            [],
        )
        .unwrap();
        let converted_db = read_entire_sqlite_db(db, Span::test_data()).unwrap();

        let expected = Value::Record {
            cols: vec!["person".to_string()],
            vals: vec![Value::List {
                vals: vec![],
                span: Span::test_data(),
            }],
            span: Span::test_data(),
        };

        assert_eq!(converted_db, expected);
    }

    #[test]
    fn can_read_null_and_non_null_data() {
        let span = Span::test_data();
        let db = Connection::open_in_memory().unwrap();

        db.execute(
            "CREATE TABLE item (
                    id     INTEGER PRIMARY KEY,
                    name   TEXT
                    )",
            [],
        )
        .unwrap();

        db.execute("INSERT INTO item (id, name) VALUES (123, NULL)", [])
            .unwrap();

        db.execute("INSERT INTO item (id, name) VALUES (456, 'foo bar')", [])
            .unwrap();

        let converted_db = read_entire_sqlite_db(db, span).unwrap();

        let expected = Value::Record {
            cols: vec!["item".to_string()],
            vals: vec![Value::List {
                vals: vec![
                    Value::Record {
                        cols: vec!["id".to_string(), "name".to_string()],
                        vals: vec![Value::Int { val: 123, span }, Value::Nothing { span }],
                        span,
                    },
                    Value::Record {
                        cols: vec!["id".to_string(), "name".to_string()],
                        vals: vec![
                            Value::Int { val: 456, span },
                            Value::String {
                                val: "foo bar".to_string(),
                                span,
                            },
                        ],
                        span,
                    },
                ],
                span,
            }],
            span,
        };

        assert_eq!(converted_db, expected);
    }
}
