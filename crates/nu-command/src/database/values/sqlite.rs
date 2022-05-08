use super::definitions::ConnectionDb;
use crate::database::values::definitions::{
    db::Db, db_column::DbColumn, db_constraint::DbConstraint, db_foreignkey::DbForeignKey,
    db_index::DbIndex, db_table::DbTable,
};
use nu_protocol::{CustomValue, PipelineData, ShellError, Span, Spanned, Value};
use rusqlite::{types::ValueRef, Connection, Row};
use serde::{Deserialize, Serialize};
use sqlparser::ast::Statement;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

const SQLITE_MAGIC_BYTES: &[u8] = "SQLite format 3\0".as_bytes();

#[derive(Debug, Serialize, Deserialize)]
pub struct SQLiteDatabase {
    // I considered storing a SQLite connection here, but decided against it because
    // 1) YAGNI, 2) it's not obvious how cloning a connection could work, 3) state
    // management gets tricky quick. Revisit this approach if we find a compelling use case.
    pub connection: ConnectionDb,
    pub statement: Option<Statement>,
}

impl SQLiteDatabase {
    pub fn new(path: &Path) -> Self {
        Self {
            connection: ConnectionDb::Path(PathBuf::from(path)),
            statement: None,
        }
    }

    pub fn try_from_path(path: &Path, span: Span) -> Result<Self, ShellError> {
        let mut file =
            File::open(path).map_err(|e| ShellError::ReadingFile(e.to_string(), span))?;

        let mut buf: [u8; 16] = [0; 16];
        file.read_exact(&mut buf)
            .map_err(|e| ShellError::ReadingFile(e.to_string(), span))
            .and_then(|_| {
                if buf == SQLITE_MAGIC_BYTES {
                    Ok(SQLiteDatabase::new(path))
                } else {
                    Err(ShellError::ReadingFile("Not a SQLite file".into(), span))
                }
            })
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(db) => Ok(Self {
                    connection: db.connection.clone(),
                    statement: db.statement.clone(),
                }),
                None => Err(ShellError::CantConvert(
                    "database".into(),
                    "non-database".into(),
                    span,
                    None,
                )),
            },
            x => Err(ShellError::CantConvert(
                "database".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(value)
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn query(&self, sql: &Spanned<String>, call_span: Span) -> Result<Value, ShellError> {
        let db = open_sqlite_db(self.connection.as_path(call_span)?, call_span)?;
        run_sql_query(db, sql).map_err(|e| {
            ShellError::GenericError(
                "Failed to query SQLite database".into(),
                e.to_string(),
                Some(sql.span),
                None,
                Vec::new(),
            )
        })
    }

    pub fn collect(&self, call_span: Span) -> Result<Value, ShellError> {
        let sql = match &self.statement {
            Some(statement) => Ok(format!("{}", statement)),
            None => Err(ShellError::GenericError(
                "Error collecting from db".into(),
                "No query found in connection".into(),
                Some(call_span),
                None,
                Vec::new(),
            )),
        }?;

        let sql = Spanned {
            item: sql,
            span: call_span,
        };

        let db = open_sqlite_db(self.connection.as_path(call_span)?, call_span)?;
        run_sql_query(db, &sql).map_err(|e| {
            ShellError::GenericError(
                "Failed to query SQLite database".into(),
                e.to_string(),
                Some(sql.span),
                None,
                Vec::new(),
            )
        })
    }

    pub fn describe(&self, span: Span) -> Value {
        let cols = vec!["connection".to_string(), "query".to_string()];
        let connection = Value::String {
            val: self.connection.to_string(),
            span,
        };

        let query = match &self.statement {
            Some(statement) => format!("{statement}"),
            None => "".into(),
        };

        let query = Value::String { val: query, span };

        Value::Record {
            cols,
            vals: vec![connection, query],
            span,
        }
    }

    pub fn open_connection(&self) -> Result<Connection, rusqlite::Error> {
        let conn = match Connection::open(self.connection.to_string()) {
            Ok(conn) => conn,
            Err(err) => return Err(err),
        };

        Ok(conn)
    }

    pub fn get_databases_and_tables(&self, conn: &Connection) -> Result<Vec<Db>, rusqlite::Error> {
        // let conn = open_connection(path)?;
        let mut db_query = conn.prepare("SELECT name FROM pragma_database_list")?;

        let databases = db_query.query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(Db::new(name, self.get_tables(conn)?))
        })?;

        let mut db_list = vec![];
        for db in databases {
            db_list.push(db?);
        }

        Ok(db_list)
    }

    pub fn get_databases(&self, conn: &Connection) -> Result<Vec<String>, rusqlite::Error> {
        let mut db_query = conn.prepare("SELECT name FROM pragma_database_list")?;

        let mut db_list = vec![];
        let _ = db_query.query_map([], |row| {
            let name: String = row.get(0)?;
            db_list.push(name);
            Ok(())
        })?;

        Ok(db_list)
    }

    pub fn get_tables(&self, conn: &Connection) -> Result<Vec<DbTable>, rusqlite::Error> {
        let mut table_names =
            conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
        let rows = table_names.query_map([], |row| row.get(0))?;
        let mut tables = Vec::new();

        for row in rows {
            let table_name: String = row?;
            tables.push(DbTable {
                name: table_name,
                create_time: None,
                update_time: None,
                engine: None,
                schema: None,
            })
        }

        Ok(tables.into_iter().collect())
    }

    fn get_column_info(&self, row: &Row) -> Result<DbColumn, rusqlite::Error> {
        let dbc = DbColumn {
            cid: row.get("cid")?,
            name: row.get("name")?,
            r#type: row.get("type")?,
            notnull: row.get("notnull")?,
            default: row.get("dflt_value")?,
            pk: row.get("pk")?,
        };
        Ok(dbc)
    }

    pub fn get_columns(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbColumn>, rusqlite::Error> {
        let mut column_names = conn.prepare(&format!(
            "SELECT * FROM pragma_table_info('{}');",
            table.name
        ))?;

        let mut columns: Vec<DbColumn> = Vec::new();
        let rows = column_names.query_and_then([], |row| self.get_column_info(row))?;

        for row in rows {
            columns.push(row?);
        }

        Ok(columns)
    }

    fn get_constraint_info(&self, row: &Row) -> Result<DbConstraint, rusqlite::Error> {
        let dbc = DbConstraint {
            name: row.get("index_name")?,
            column_name: row.get("column_name")?,
            origin: row.get("origin")?,
        };
        Ok(dbc)
    }

    pub fn get_constraints(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbConstraint>, rusqlite::Error> {
        let mut column_names = conn.prepare(&format!(
            "
            SELECT
                p.origin,
                s.name AS index_name,
                i.name AS column_name
            FROM
                sqlite_master s
                JOIN pragma_index_list(s.tbl_name) p ON s.name = p.name,
                pragma_index_info(s.name) i
            WHERE
                s.type = 'index'
                AND tbl_name = '{}'
                AND NOT p.origin = 'c'
            ",
            &table.name
        ))?;

        let mut constraints: Vec<DbConstraint> = Vec::new();
        let rows = column_names.query_and_then([], |row| self.get_constraint_info(row))?;

        for row in rows {
            constraints.push(row?);
        }

        Ok(constraints)
    }

    fn get_foreign_keys_info(&self, row: &Row) -> Result<DbForeignKey, rusqlite::Error> {
        let dbc = DbForeignKey {
            column_name: row.get("from")?,
            ref_table: row.get("table")?,
            ref_column: row.get("to")?,
        };
        Ok(dbc)
    }

    pub fn get_foreign_keys(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbForeignKey>, rusqlite::Error> {
        let mut column_names = conn.prepare(&format!(
            "SELECT p.`from`, p.`to`, p.`table` FROM pragma_foreign_key_list('{}') p",
            &table.name
        ))?;

        let mut foreign_keys: Vec<DbForeignKey> = Vec::new();
        let rows = column_names.query_and_then([], |row| self.get_foreign_keys_info(row))?;

        for row in rows {
            foreign_keys.push(row?);
        }

        Ok(foreign_keys)
    }

    fn get_index_info(&self, row: &Row) -> Result<DbIndex, rusqlite::Error> {
        let dbc = DbIndex {
            name: row.get("index_name")?,
            column_name: row.get("name")?,
            seqno: row.get("seqno")?,
        };
        Ok(dbc)
    }

    pub fn get_indexes(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbIndex>, rusqlite::Error> {
        let mut column_names = conn.prepare(&format!(
            "
            SELECT
                m.name AS index_name,
                p.*
            FROM
                sqlite_master m,
                pragma_index_info(m.name) p
            WHERE
                m.type = 'index'
                AND m.tbl_name = '{}'
            ",
            &table.name,
        ))?;

        let mut indexes: Vec<DbIndex> = Vec::new();
        let rows = column_names.query_and_then([], |row| self.get_index_info(row))?;

        for row in rows {
            indexes.push(row?);
        }

        Ok(indexes)
    }
}

impl CustomValue for SQLiteDatabase {
    fn clone_value(&self, span: Span) -> Value {
        let cloned = SQLiteDatabase {
            connection: self.connection.clone(),
            statement: self.statement.clone(),
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
        let db = open_sqlite_db(self.connection.as_path(span)?, span)?;
        read_entire_sqlite_db(db, span).map_err(|e| {
            ShellError::GenericError(
                "Failed to read from SQLite database".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            )
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(&self, _count: usize, span: Span) -> Result<Value, ShellError> {
        // In theory we could support this, but tables don't have an especially well-defined order
        Err(ShellError::IncompatiblePathAccess("SQLite databases do not support integer-indexed access. Try specifying a table name instead".into(), span))
    }

    fn follow_path_string(&self, _column_name: String, span: Span) -> Result<Value, ShellError> {
        let db = open_sqlite_db(self.connection.as_path(span)?, span)?;

        read_single_table(db, _column_name, span).map_err(|e| {
            ShellError::GenericError(
                "Failed to read from SQLite database".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            )
        })
    }

    fn typetag_name(&self) -> &'static str {
        "SQLiteDatabase"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

fn open_sqlite_db(path: &Path, call_span: Span) -> Result<Connection, nu_protocol::ShellError> {
    let path = path.to_string_lossy().to_string();

    Connection::open(path).map_err(|e| {
        ShellError::GenericError(
            "Failed to open SQLite database".into(),
            e.to_string(),
            Some(call_span),
            None,
            Vec::new(),
        )
    })
}

fn run_sql_query(conn: Connection, sql: &Spanned<String>) -> Result<Value, rusqlite::Error> {
    let mut stmt = conn.prepare(&sql.item)?;
    let results = stmt.query([])?;

    let nu_records = results
        .mapped(|row| Result::Ok(convert_sqlite_row_to_nu_value(row, sql.span)))
        .into_iter()
        .collect::<Result<Vec<Value>, rusqlite::Error>>()?;

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
    let results = stmt.query([])?;

    let nu_records = results
        .mapped(|row| Result::Ok(convert_sqlite_row_to_nu_value(row, call_span)))
        .into_iter()
        .collect::<Result<Vec<Value>, rusqlite::Error>>()?;

    Ok(Value::List {
        vals: nu_records,
        span: call_span,
    })
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

pub fn convert_sqlite_row_to_nu_value(row: &Row, span: Span) -> Value {
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

pub fn convert_sqlite_value_to_nu_value(value: ValueRef, span: Span) -> Value {
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
        let db = open_connection_in_memory().unwrap();
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
        let db = open_connection_in_memory().unwrap();

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
        let db = open_connection_in_memory().unwrap();

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

pub fn open_connection_in_memory() -> Result<Connection, ShellError> {
    let db = match Connection::open_in_memory() {
        Ok(conn) => conn,
        Err(err) => {
            return Err(ShellError::GenericError(
                "Failed to open SQLite connection in memory".into(),
                err.to_string(),
                Some(Span::test_data()),
                None,
                Vec::new(),
            ))
        }
    };

    Ok(db)
}
