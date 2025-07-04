use super::definitions::{
    db_column::DbColumn, db_constraint::DbConstraint, db_foreignkey::DbForeignKey,
    db_index::DbIndex, db_table::DbTable,
};
use nu_protocol::{
    CustomValue, PipelineData, Record, ShellError, Signals, Span, Spanned, Value,
    shell_error::io::IoError,
};
use rusqlite::{
    Connection, DatabaseName, Error as SqliteError, OpenFlags, Row, Statement, ToSql,
    types::ValueRef,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

const SQLITE_MAGIC_BYTES: &[u8] = "SQLite format 3\0".as_bytes();
pub const MEMORY_DB: &str = "file:memdb1?mode=memory&cache=shared";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SQLiteDatabase {
    // I considered storing a SQLite connection here, but decided against it because
    // 1) YAGNI, 2) it's not obvious how cloning a connection could work, 3) state
    // management gets tricky quick. Revisit this approach if we find a compelling use case.
    pub path: PathBuf,
    #[serde(skip, default = "Signals::empty")]
    // this understandably can't be serialized. think that's OK, I'm not aware of a
    // reason why a CustomValue would be serialized outside of a plugin
    signals: Signals,
}

impl SQLiteDatabase {
    pub fn new(path: &Path, signals: Signals) -> Self {
        Self {
            path: PathBuf::from(path),
            signals,
        }
    }

    pub fn try_from_path(path: &Path, span: Span, signals: Signals) -> Result<Self, ShellError> {
        let mut file = File::open(path).map_err(|e| IoError::new(e, span, PathBuf::from(path)))?;

        let mut buf: [u8; 16] = [0; 16];
        file.read_exact(&mut buf)
            .map_err(|e| ShellError::Io(IoError::new(e, span, PathBuf::from(path))))
            .and_then(|_| {
                if buf == SQLITE_MAGIC_BYTES {
                    Ok(SQLiteDatabase::new(path, signals))
                } else {
                    Err(ShellError::GenericError {
                        error: "Not a SQLite file".into(),
                        msg: format!("Could not read '{}' as SQLite file", path.display()),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    })
                }
            })
    }

    pub fn try_from_value(value: Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => match val.as_any().downcast_ref::<Self>() {
                Some(db) => Ok(Self {
                    path: db.path.clone(),
                    signals: db.signals.clone(),
                }),
                None => Err(ShellError::CantConvert {
                    to_type: "database".into(),
                    from_type: "non-database".into(),
                    span,
                    help: None,
                }),
            },
            x => Err(ShellError::CantConvert {
                to_type: "database".into(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span)?;
        Self::try_from_value(value)
    }

    pub fn into_value(self, span: Span) -> Value {
        let db = Box::new(self);
        Value::custom(db, span)
    }

    pub fn query(
        &self,
        sql: &Spanned<String>,
        params: NuSqlParams,
        call_span: Span,
    ) -> Result<Value, ShellError> {
        let conn = open_sqlite_db(&self.path, call_span)?;
        let stream = run_sql_query(conn, sql, params, &self.signals)
            .map_err(|e| e.into_shell_error(sql.span, "Failed to query SQLite database"))?;

        Ok(stream)
    }

    pub fn open_connection(&self) -> Result<Connection, ShellError> {
        if self.path == PathBuf::from(MEMORY_DB) {
            open_connection_in_memory_custom()
        } else {
            let conn = Connection::open(&self.path).map_err(|e| ShellError::GenericError {
                error: "Failed to open SQLite database from open_connection".into(),
                msg: e.to_string(),
                span: None,
                help: None,
                inner: vec![],
            })?;
            conn.busy_handler(Some(SQLiteDatabase::sleeper))
                .map_err(|e| ShellError::GenericError {
                    error: "Failed to set busy handler for SQLite database".into(),
                    msg: e.to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                })?;
            Ok(conn)
        }
    }

    fn sleeper(attempts: i32) -> bool {
        log::warn!("SQLITE_BUSY, retrying after 250ms (attempt {})", attempts);
        std::thread::sleep(std::time::Duration::from_millis(250));
        true
    }

    pub fn get_tables(&self, conn: &Connection) -> Result<Vec<DbTable>, SqliteError> {
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

    pub fn drop_all_tables(&self, conn: &Connection) -> Result<(), SqliteError> {
        let tables = self.get_tables(conn)?;

        for table in tables {
            conn.execute(&format!("DROP TABLE {}", table.name), [])?;
        }

        Ok(())
    }

    pub fn export_in_memory_database_to_file(
        &self,
        conn: &Connection,
        filename: String,
    ) -> Result<(), SqliteError> {
        //vacuum main into 'c:\\temp\\foo.db'
        conn.execute(&format!("vacuum main into '{filename}'"), [])?;

        Ok(())
    }

    pub fn backup_database_to_file(
        &self,
        conn: &Connection,
        filename: String,
    ) -> Result<(), SqliteError> {
        conn.backup(DatabaseName::Main, Path::new(&filename), None)?;
        Ok(())
    }

    pub fn restore_database_from_file(
        &self,
        conn: &mut Connection,
        filename: String,
    ) -> Result<(), SqliteError> {
        conn.restore(
            DatabaseName::Main,
            Path::new(&filename),
            Some(|p: rusqlite::backup::Progress| {
                let percent = if p.pagecount == 0 {
                    100
                } else {
                    (p.pagecount - p.remaining) * 100 / p.pagecount
                };
                if percent % 10 == 0 {
                    log::trace!("Restoring: {} %", percent);
                }
            }),
        )?;
        Ok(())
    }

    fn get_column_info(&self, row: &Row) -> Result<DbColumn, SqliteError> {
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
    ) -> Result<Vec<DbColumn>, SqliteError> {
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

    fn get_constraint_info(&self, row: &Row) -> Result<DbConstraint, SqliteError> {
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
    ) -> Result<Vec<DbConstraint>, SqliteError> {
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

    fn get_foreign_keys_info(&self, row: &Row) -> Result<DbForeignKey, SqliteError> {
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
    ) -> Result<Vec<DbForeignKey>, SqliteError> {
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

    fn get_index_info(&self, row: &Row) -> Result<DbIndex, SqliteError> {
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
    ) -> Result<Vec<DbIndex>, SqliteError> {
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
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let db = open_sqlite_db(&self.path, span)?;
        read_entire_sqlite_db(db, span, &self.signals)
            .map_err(|e| e.into_shell_error(span, "Failed to read from SQLite database"))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        _index: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        // In theory we could support this, but tables don't have an especially well-defined order
        Err(ShellError::IncompatiblePathAccess { type_name: "SQLite databases do not support integer-indexed access. Try specifying a table name instead".into(), span: path_span })
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let db = open_sqlite_db(&self.path, path_span)?;
        read_single_table(db, column_name, path_span, &self.signals)
            .map_err(|e| e.into_shell_error(path_span, "Failed to read from SQLite database"))
    }

    fn typetag_name(&self) -> &'static str {
        "SQLiteDatabase"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

pub fn open_sqlite_db(path: &Path, call_span: Span) -> Result<Connection, ShellError> {
    if path.to_string_lossy() == MEMORY_DB {
        open_connection_in_memory_custom()
    } else {
        let path = path.to_string_lossy().to_string();
        Connection::open(path).map_err(|err| ShellError::GenericError {
            error: "Failed to open SQLite database".into(),
            msg: err.to_string(),
            span: Some(call_span),
            help: None,
            inner: Vec::new(),
        })
    }
}

fn run_sql_query(
    conn: Connection,
    sql: &Spanned<String>,
    params: NuSqlParams,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let stmt = conn.prepare(&sql.item)?;
    prepared_statement_to_nu_list(stmt, params, sql.span, signals)
}

// This is taken from to text local_into_string but tweaks it a bit so that certain formatting does not happen
pub fn value_to_sql(value: Value) -> Result<Box<dyn rusqlite::ToSql>, ShellError> {
    Ok(match value {
        Value::Bool { val, .. } => Box::new(val),
        Value::Int { val, .. } => Box::new(val),
        Value::Float { val, .. } => Box::new(val),
        Value::Filesize { val, .. } => Box::new(val.get()),
        Value::Duration { val, .. } => Box::new(val),
        Value::Date { val, .. } => Box::new(val),
        Value::String { val, .. } => Box::new(val),
        Value::Binary { val, .. } => Box::new(val),
        Value::Nothing { .. } => Box::new(rusqlite::types::Null),
        val => {
            return Err(ShellError::OnlySupportsThisInputType {
                exp_input_type:
                    "bool, int, float, filesize, duration, date, string, nothing, binary".into(),
                wrong_type: val.get_type().to_string(),
                dst_span: Span::unknown(),
                src_span: val.span(),
            });
        }
    })
}

pub fn values_to_sql(
    values: impl IntoIterator<Item = Value>,
) -> Result<Vec<Box<dyn rusqlite::ToSql>>, ShellError> {
    values
        .into_iter()
        .map(value_to_sql)
        .collect::<Result<Vec<_>, _>>()
}

pub enum NuSqlParams {
    List(Vec<Box<dyn ToSql>>),
    Named(Vec<(String, Box<dyn ToSql>)>),
}

impl Default for NuSqlParams {
    fn default() -> Self {
        NuSqlParams::List(Vec::new())
    }
}

pub fn nu_value_to_params(value: Value) -> Result<NuSqlParams, ShellError> {
    match value {
        Value::Record { val, .. } => {
            let mut params = Vec::with_capacity(val.len());

            for (mut column, value) in val.into_owned().into_iter() {
                let sql_type_erased = value_to_sql(value)?;

                if !column.starts_with([':', '@', '$']) {
                    column.insert(0, ':');
                }

                params.push((column, sql_type_erased));
            }

            Ok(NuSqlParams::Named(params))
        }
        Value::List { vals, .. } => {
            let mut params = Vec::with_capacity(vals.len());

            for value in vals.into_iter() {
                let sql_type_erased = value_to_sql(value)?;

                params.push(sql_type_erased);
            }

            Ok(NuSqlParams::List(params))
        }

        // We accept no parameters
        Value::Nothing { .. } => Ok(NuSqlParams::default()),

        _ => Err(ShellError::TypeMismatch {
            err_message: "Invalid parameters value: expected record or list".to_string(),
            span: value.span(),
        }),
    }
}

#[derive(Debug)]
enum SqliteOrShellError {
    SqliteError(SqliteError),
    ShellError(ShellError),
}

impl From<SqliteError> for SqliteOrShellError {
    fn from(error: SqliteError) -> Self {
        Self::SqliteError(error)
    }
}

impl From<ShellError> for SqliteOrShellError {
    fn from(error: ShellError) -> Self {
        Self::ShellError(error)
    }
}

impl SqliteOrShellError {
    fn into_shell_error(self, span: Span, msg: &str) -> ShellError {
        match self {
            Self::SqliteError(err) => ShellError::GenericError {
                error: msg.into(),
                msg: err.to_string(),
                span: Some(span),
                help: None,
                inner: Vec::new(),
            },
            Self::ShellError(err) => err,
        }
    }
}

fn read_single_table(
    conn: Connection,
    table_name: String,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    // TODO: Should use params here?
    let stmt = conn.prepare(&format!("SELECT * FROM [{table_name}]"))?;
    prepared_statement_to_nu_list(stmt, NuSqlParams::default(), call_span, signals)
}

fn prepared_statement_to_nu_list(
    mut stmt: Statement,
    params: NuSqlParams,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let column_names = stmt
        .column_names()
        .into_iter()
        .map(String::from)
        .collect::<Vec<String>>();

    // I'm very sorry for this repetition
    // I tried scoping the match arms to the query_map alone, but lifetime and closure reference escapes
    // got heavily in the way
    let row_values = match params {
        NuSqlParams::List(params) => {
            let refs: Vec<&dyn ToSql> = params.iter().map(|value| (&**value)).collect();

            let row_results = stmt.query_map(refs.as_slice(), |row| {
                Ok(convert_sqlite_row_to_nu_value(
                    row,
                    call_span,
                    &column_names,
                ))
            })?;

            // we collect all rows before returning them. Not ideal but it's hard/impossible to return a stream from a CustomValue
            let mut row_values = vec![];

            for row_result in row_results {
                signals.check(call_span)?;
                if let Ok(row_value) = row_result {
                    row_values.push(row_value);
                }
            }

            row_values
        }
        NuSqlParams::Named(pairs) => {
            let refs: Vec<_> = pairs
                .iter()
                .map(|(column, value)| (column.as_str(), &**value))
                .collect();

            let row_results = stmt.query_map(refs.as_slice(), |row| {
                Ok(convert_sqlite_row_to_nu_value(
                    row,
                    call_span,
                    &column_names,
                ))
            })?;

            // we collect all rows before returning them. Not ideal but it's hard/impossible to return a stream from a CustomValue
            let mut row_values = vec![];

            for row_result in row_results {
                signals.check(call_span)?;
                if let Ok(row_value) = row_result {
                    row_values.push(row_value);
                }
            }

            row_values
        }
    };

    Ok(Value::list(row_values, call_span))
}

fn read_entire_sqlite_db(
    conn: Connection,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let mut tables = Record::new();

    let mut get_table_names =
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let rows = get_table_names.query_map([], |row| row.get(0))?;

    for row in rows {
        let table_name: String = row?;
        // TODO: Should use params here?
        let table_stmt = conn.prepare(&format!("select * from [{table_name}]"))?;
        let rows =
            prepared_statement_to_nu_list(table_stmt, NuSqlParams::default(), call_span, signals)?;
        tables.push(table_name, rows);
    }

    Ok(Value::record(tables, call_span))
}

pub fn convert_sqlite_row_to_nu_value(row: &Row, span: Span, column_names: &[String]) -> Value {
    let record = column_names
        .iter()
        .enumerate()
        .map(|(i, col)| {
            (
                col.clone(),
                convert_sqlite_value_to_nu_value(row.get_ref_unwrap(i), span),
            )
        })
        .collect();

    Value::record(record, span)
}

pub fn convert_sqlite_value_to_nu_value(value: ValueRef, span: Span) -> Value {
    match value {
        ValueRef::Null => Value::nothing(span),
        ValueRef::Integer(i) => Value::int(i, span),
        ValueRef::Real(f) => Value::float(f, span),
        ValueRef::Text(buf) => {
            let s = match std::str::from_utf8(buf) {
                Ok(v) => v,
                Err(_) => return Value::error(ShellError::NonUtf8 { span }, span),
            };
            Value::string(s.to_string(), span)
        }
        ValueRef::Blob(u) => Value::binary(u.to_vec(), span),
    }
}

pub fn open_connection_in_memory_custom() -> Result<Connection, ShellError> {
    let flags = OpenFlags::default();
    let conn =
        Connection::open_with_flags(MEMORY_DB, flags).map_err(|e| ShellError::GenericError {
            error: "Failed to open SQLite custom connection in memory".into(),
            msg: e.to_string(),
            span: Some(Span::test_data()),
            help: None,
            inner: vec![],
        })?;
    conn.busy_handler(Some(SQLiteDatabase::sleeper))
        .map_err(|e| ShellError::GenericError {
            error: "Failed to set busy handler for SQLite custom connection in memory".into(),
            msg: e.to_string(),
            span: Some(Span::test_data()),
            help: None,
            inner: vec![],
        })?;
    Ok(conn)
}

pub fn open_connection_in_memory() -> Result<Connection, ShellError> {
    Connection::open_in_memory().map_err(|e| ShellError::GenericError {
        error: "Failed to open SQLite standard connection in memory".into(),
        msg: e.to_string(),
        span: Some(Span::test_data()),
        help: None,
        inner: vec![],
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::record;

    #[test]
    fn can_read_empty_db() {
        let db = open_connection_in_memory().unwrap();
        let converted_db = read_entire_sqlite_db(db, Span::test_data(), &Signals::empty()).unwrap();

        let expected = Value::test_record(Record::new());

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
        let converted_db = read_entire_sqlite_db(db, Span::test_data(), &Signals::empty()).unwrap();

        let expected = Value::test_record(record! {
            "person" => Value::test_list(vec![]),
        });

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

        let converted_db = read_entire_sqlite_db(db, span, &Signals::empty()).unwrap();

        let expected = Value::test_record(record! {
            "item" => Value::test_list(
                vec![
                    Value::test_record(record! {
                        "id" =>   Value::test_int(123),
                        "name" => Value::nothing(span),
                    }),
                    Value::test_record(record! {
                        "id" =>   Value::test_int(456),
                        "name" => Value::test_string("foo bar"),
                    }),
                ]
            ),
        });

        assert_eq!(converted_db, expected);
    }
}
