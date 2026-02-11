use super::definitions::{
    db_column::DbColumn, db_constraint::DbConstraint, db_foreignkey::DbForeignKey,
    db_index::DbIndex, db_table::DbTable,
};
use nu_protocol::{
    CustomValue, IntoPipelineData, PipelineData, Record, ShellError, Signals, Span, Spanned, Value,
    ast, casing::Casing, engine::EngineState, shell_error::io::IoError,
};
use rusqlite::{
    Connection, Error as SqliteError, OpenFlags, Row, Statement, ToSql, types::ValueRef,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

const SQLITE_MAGIC_BYTES: &[u8] = "SQLite format 3\0".as_bytes();
pub const MEMORY_DB: &str = "file:memdb1?mode=memory&cache=shared";
const DATABASE_NAME: &str = "main";

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
        if self.path.to_string_lossy() == MEMORY_DB {
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
        log::warn!("SQLITE_BUSY, retrying after 250ms (attempt {attempts})");
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
        conn.backup(DATABASE_NAME, Path::new(&filename), None)?;
        Ok(())
    }

    pub fn restore_database_from_file(
        &self,
        conn: &mut Connection,
        filename: String,
    ) -> Result<(), SqliteError> {
        conn.restore(
            DATABASE_NAME,
            Path::new(&filename),
            Some(|p: rusqlite::backup::Progress| {
                let percent = if p.pagecount == 0 {
                    100
                } else {
                    (p.pagecount - p.remaining) * 100 / p.pagecount
                };
                if percent % 10 == 0 {
                    log::trace!("Restoring: {percent} %");
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
        _optional: bool,
    ) -> Result<Value, ShellError> {
        // In theory we could support this, but tables don't have an especially well-defined order
        Err(ShellError::IncompatiblePathAccess { type_name: "SQLite databases do not support integer-indexed access. Try specifying a table name instead".into(), span: path_span })
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        column_name: String,
        path_span: Span,
        _optional: bool,
        _casing: Casing,
    ) -> Result<Value, ShellError> {
        // Return a lazy SQLiteQueryBuilder instead of executing the query immediately
        let table = SQLiteQueryBuilder::new(self.path.clone(), column_name, self.signals.clone());
        Ok(Value::custom(Box::new(table), path_span))
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
pub fn value_to_sql(
    engine_state: &EngineState,
    value: Value,
    call_span: Span,
) -> Result<Box<dyn rusqlite::ToSql>, ShellError> {
    match value {
        Value::Bool { val, .. } => Ok(Box::new(val)),
        Value::Int { val, .. } => Ok(Box::new(val)),
        Value::Float { val, .. } => Ok(Box::new(val)),
        Value::Filesize { val, .. } => Ok(Box::new(val.get())),
        Value::Duration { val, .. } => Ok(Box::new(val)),
        Value::Date { val, .. } => Ok(Box::new(val)),
        Value::String { val, .. } => Ok(Box::new(val)),
        Value::Binary { val, .. } => Ok(Box::new(val)),
        Value::Nothing { .. } => Ok(Box::new(rusqlite::types::Null)),
        val => {
            let json_value = crate::value_to_json_value(engine_state, &val, call_span, false)?;
            match nu_json::to_string_raw(&json_value) {
                Ok(s) => Ok(Box::new(s)),
                Err(err) => Err(ShellError::CantConvert {
                    to_type: "JSON".into(),
                    from_type: val.get_type().to_string(),
                    span: val.span(),
                    help: Some(err.to_string()),
                }),
            }
        }
    }
}

pub fn values_to_sql(
    engine_state: &EngineState,
    values: impl IntoIterator<Item = Value>,
    call_span: Span,
) -> Result<Vec<Box<dyn rusqlite::ToSql>>, ShellError> {
    values
        .into_iter()
        .map(|v| value_to_sql(engine_state, v, call_span))
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

pub fn nu_value_to_params(
    engine_state: &EngineState,
    value: Value,
    call_span: Span,
) -> Result<NuSqlParams, ShellError> {
    match value {
        Value::Record { val, .. } => {
            let mut params = Vec::with_capacity(val.len());

            for (mut column, value) in val.into_owned().into_iter() {
                let sql_type_erased = value_to_sql(engine_state, value, call_span)?;

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
                let sql_type_erased = value_to_sql(engine_state, value, call_span)?;

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

/// The SQLite type behind a query column returned as some raw type (e.g. 'text')
#[derive(Clone, Copy)]
pub enum DeclType {
    Json,
    Jsonb,
}

impl DeclType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "JSON" => Some(DeclType::Json),
            "JSONB" => Some(DeclType::Jsonb),
            _ => None, // We are only special-casing JSON(B) columns for now
        }
    }
}

/// A column out of an SQLite query, together with its type
pub struct TypedColumn {
    pub name: String,
    pub decl_type: Option<DeclType>,
}

impl TypedColumn {
    pub fn from_rusqlite_column(c: &rusqlite::Column) -> Self {
        Self {
            name: c.name().to_owned(),
            decl_type: c.decl_type().and_then(DeclType::from_str),
        }
    }
}

fn prepared_statement_to_nu_list(
    mut stmt: Statement,
    params: NuSqlParams,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let columns: Vec<TypedColumn> = stmt
        .columns()
        .iter()
        .map(TypedColumn::from_rusqlite_column)
        .collect();

    // I'm very sorry for this repetition
    // I tried scoping the match arms to the query_map alone, but lifetime and closure reference escapes
    // got heavily in the way
    let row_values = match params {
        NuSqlParams::List(params) => {
            let refs: Vec<&dyn ToSql> = params.iter().map(|value| &**value).collect();

            let row_results = stmt.query_map(refs.as_slice(), |row| {
                Ok(convert_sqlite_row_to_nu_value(row, call_span, &columns))
            })?;

            // we collect all rows before returning them. Not ideal but it's hard/impossible to return a stream from a CustomValue
            let mut row_values = vec![];

            for row_result in row_results {
                signals.check(&call_span)?;
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
                Ok(convert_sqlite_row_to_nu_value(row, call_span, &columns))
            })?;

            // we collect all rows before returning them. Not ideal but it's hard/impossible to return a stream from a CustomValue
            let mut row_values = vec![];

            for row_result in row_results {
                signals.check(&call_span)?;
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

pub fn convert_sqlite_row_to_nu_value(row: &Row, span: Span, columns: &[TypedColumn]) -> Value {
    let record = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            (
                col.name.clone(),
                convert_sqlite_value_to_nu_value(row.get_ref_unwrap(i), col.decl_type, span),
            )
        })
        .collect();

    Value::record(record, span)
}

pub fn convert_sqlite_value_to_nu_value(
    value: ValueRef,
    decl_type: Option<DeclType>,
    span: Span,
) -> Value {
    match value {
        ValueRef::Null => Value::nothing(span),
        ValueRef::Integer(i) => Value::int(i, span),
        ValueRef::Real(f) => Value::float(f, span),
        ValueRef::Text(buf) => match (std::str::from_utf8(buf), decl_type) {
            (Ok(txt), Some(DeclType::Json | DeclType::Jsonb)) => {
                match crate::convert_json_string_to_value(txt, span) {
                    Ok(val) => val,
                    Err(err) => Value::error(err, span),
                }
            }
            (Ok(txt), _) => Value::string(txt.to_string(), span),
            (Err(_), _) => Value::error(ShellError::NonUtf8 { span }, span),
        },
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

/// A lazy query builder for SQLite tables, allowing SQL pushdown optimizations
/// for commands like `length`, `select`, `first`, and `last`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SQLiteQueryBuilder {
    pub db_path: PathBuf,
    pub table_name: String,
    pub sql_select: Option<String>, // e.g., "column1, column2" or "*" for all
    pub sql_where: Option<String>,  // e.g., "column = ?"
    pub sql_params: Vec<String>,    // parameters for the where clause
    pub sql_order_by: Option<String>, // e.g., "id DESC"
    pub sql_limit: Option<i64>,
    #[serde(skip, default = "Signals::empty")]
    signals: Signals,
}

impl SQLiteQueryBuilder {
    pub fn new(db_path: PathBuf, table_name: String, signals: Signals) -> Self {
        Self {
            db_path,
            table_name,
            sql_select: None,
            sql_where: None,
            sql_params: Vec::new(),
            sql_order_by: None,
            sql_limit: None,
            signals,
        }
    }

    pub fn with_select(mut self, select: String) -> Self {
        self.sql_select = Some(select);
        self
    }

    pub fn with_where(mut self, where_clause: String, params: Vec<String>) -> Self {
        self.sql_where = Some(where_clause);
        self.sql_params = params;
        self
    }

    pub fn with_order_by(mut self, order_by: String) -> Self {
        self.sql_order_by = Some(order_by);
        self
    }

    pub fn with_limit(mut self, limit: i64) -> Self {
        self.sql_limit = Some(limit);
        self
    }

    pub fn build_sql(&self) -> String {
        let select = self.sql_select.as_deref().unwrap_or("*");
        let mut sql = format!("SELECT {} FROM [{}]", select, self.table_name);

        if let Some(where_clause) = &self.sql_where {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        if let Some(order_by) = &self.sql_order_by {
            sql.push_str(&format!(" ORDER BY {}", order_by));
        }

        if let Some(limit) = self.sql_limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        sql
    }

    pub fn execute(&self, call_span: Span) -> Result<PipelineData, ShellError> {
        let conn = open_sqlite_db(&self.db_path, call_span)?;
        let sql = self.build_sql();
        let params = NuSqlParams::List(Vec::new()); // FIXME: handle params properly
        run_sql_query(
            conn,
            &Spanned {
                item: sql,
                span: call_span,
            },
            params,
            &self.signals,
        )
        .map(IntoPipelineData::into_pipeline_data)
        .map_err(|e| e.into_shell_error(call_span, "Failed to execute query"))
    }

    pub fn count(&self, call_span: Span) -> Result<i64, ShellError> {
        let conn = open_sqlite_db(&self.db_path, call_span)?;
        let mut sql = format!("SELECT COUNT(*) FROM [{}]", self.table_name);
        if let Some(where_clause) = &self.sql_where {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }
        let mut stmt = conn.prepare(&sql).map_err(|e| ShellError::GenericError {
            error: "Failed to prepare count query".into(),
            msg: e.to_string(),
            span: Some(call_span),
            help: None,
            inner: vec![],
        })?;
        let params: Vec<Box<dyn ToSql>> = self
            .sql_params
            .iter()
            .map(|s| Box::new(s.clone()) as Box<dyn ToSql>)
            .collect();
        let count: i64 = stmt
            .query_row(rusqlite::params_from_iter(params), |row| row.get(0))
            .map_err(|e| ShellError::GenericError {
                error: "Failed to execute count query".into(),
                msg: e.to_string(),
                span: Some(call_span),
                help: None,
                inner: vec![],
            })?;
        Ok(count)
    }
}

impl CustomValue for SQLiteQueryBuilder {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        "SQLiteQueryBuilder".to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        self.execute(span).and_then(|pd| pd.into_value(span))
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
        index: usize,
        path_span: Span,
        optional: bool,
    ) -> Result<Value, ShellError> {
        // Execute and then index - this could be optimized with LIMIT/OFFSET later
        let data = self.to_base_value(path_span)?;
        data.follow_cell_path(&[ast::PathMember::Int {
            val: index,
            span: path_span,
            optional,
        }])
        .map(|v| v.into_owned())
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        column_name: String,
        path_span: Span,
        _optional: bool,
        _: Casing,
    ) -> Result<Value, ShellError> {
        // For now, just execute and get the column - this could be optimized later
        let data = self.to_base_value(path_span)?;
        data.follow_cell_path(&[ast::PathMember::String {
            val: column_name,
            span: path_span,
            optional: false,
            casing: Casing::default(),
        }])
        .map(|v| v.into_owned())
    }

    fn typetag_name(&self) -> &'static str {
        "SQLiteQueryBuilder"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn is_iterable(&self) -> bool {
        true
    }
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

    #[test]
    fn sqlite_table_build_sql_combined() {
        let table = SQLiteQueryBuilder::new(
            PathBuf::from(":memory:"),
            "test".to_string(),
            Signals::empty(),
        )
        .with_select("col1".to_string())
        .with_where("col2 = ?".to_string(), vec!["val".to_string()])
        .with_order_by("col1".to_string())
        .with_limit(5);
        assert_eq!(
            table.build_sql(),
            "SELECT col1 FROM [test] WHERE col2 = ? ORDER BY col1 LIMIT 5"
        );
    }

    #[test]
    fn sqlite_table_count_integration() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let signals = Signals::empty();

        // Create a test DB with data
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE test (id INTEGER, name TEXT)", [])
                .unwrap();
            for i in 0..10 {
                conn.execute(
                    "INSERT INTO test (id, name) VALUES (?, ?)",
                    rusqlite::params![i, format!("name{}", i)],
                )
                .unwrap();
            }
        }

        let table = SQLiteQueryBuilder::new(db_path, "test".to_string(), signals);
        let count = table.count(Span::test_data()).unwrap();
        assert_eq!(count, 10);
    }

    #[test]
    fn sqlite_table_execute_integration() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let signals = Signals::empty();

        // Create a test DB with data
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE test (id INTEGER, name TEXT)", [])
                .unwrap();
            conn.execute("INSERT INTO test (id, name) VALUES (1, 'first')", [])
                .unwrap();
            conn.execute("INSERT INTO test (id, name) VALUES (2, 'second')", [])
                .unwrap();
        }

        let table = SQLiteQueryBuilder::new(db_path, "test".to_string(), signals);
        let result = table.execute(Span::test_data()).unwrap();
        let value = result.into_value(Span::test_data()).unwrap();

        if let Value::List { vals, .. } = value {
            assert_eq!(vals.len(), 2);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn sqlite_table_first_integration() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let signals = Signals::empty();

        // Create a test DB with data
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE test (id INTEGER, name TEXT)", [])
                .unwrap();
            for i in 0..5 {
                conn.execute(
                    "INSERT INTO test (id, name) VALUES (?, ?)",
                    rusqlite::params![i, format!("name{}", i)],
                )
                .unwrap();
            }
        }

        let table = SQLiteQueryBuilder::new(db_path, "test".to_string(), signals).with_limit(2);
        let result = table.execute(Span::test_data()).unwrap();
        let value = result.into_value(Span::test_data()).unwrap();

        if let Value::List { vals, .. } = value {
            assert_eq!(vals.len(), 2);
            // Check first two ids
            if let Value::Record { val: record, .. } = &vals[0] {
                assert_eq!(record.get("id"), Some(&Value::int(0, Span::test_data())));
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn sqlite_table_last_integration() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_path_buf();
        let signals = Signals::empty();

        // Create a test DB with data
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute("CREATE TABLE test (id INTEGER, name TEXT)", [])
                .unwrap();
            for i in 0..5 {
                conn.execute(
                    "INSERT INTO test (id, name) VALUES (?, ?)",
                    rusqlite::params![i, format!("name{}", i)],
                )
                .unwrap();
            }
        }

        let table = SQLiteQueryBuilder::new(db_path, "test".to_string(), signals)
            .with_order_by("rowid DESC".to_string())
            .with_limit(2);
        let result = table.execute(Span::test_data()).unwrap();
        let value = result.into_value(Span::test_data()).unwrap();

        if let Value::List { vals, .. } = value {
            assert_eq!(vals.len(), 2);
            // Check last two ids (since DESC, first in result is highest)
            if let Value::Record { val: record, .. } = &vals[0] {
                assert_eq!(record.get("id"), Some(&Value::int(4, Span::test_data())));
            }
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn sqlite_table_build_sql_with_select() {
        let table = SQLiteQueryBuilder::new(
            PathBuf::from(":memory:"),
            "test".to_string(),
            Signals::empty(),
        )
        .with_select("col1, col2".to_string());
        assert_eq!(table.build_sql(), "SELECT col1, col2 FROM [test]");
    }

    #[test]
    fn sqlite_table_build_sql_with_where() {
        let table = SQLiteQueryBuilder::new(
            PathBuf::from(":memory:"),
            "test".to_string(),
            Signals::empty(),
        )
        .with_where("col = ?".to_string(), vec!["val".to_string()]);
        assert_eq!(table.build_sql(), "SELECT * FROM [test] WHERE col = ?");
    }

    #[test]
    fn sqlite_table_build_sql_with_order_by() {
        let table = SQLiteQueryBuilder::new(
            PathBuf::from(":memory:"),
            "test".to_string(),
            Signals::empty(),
        )
        .with_order_by("id DESC".to_string());
        assert_eq!(table.build_sql(), "SELECT * FROM [test] ORDER BY id DESC");
    }

    #[test]
    fn sqlite_table_build_sql_with_limit() {
        let table = SQLiteQueryBuilder::new(
            PathBuf::from(":memory:"),
            "test".to_string(),
            Signals::empty(),
        )
        .with_limit(10);
        assert_eq!(table.build_sql(), "SELECT * FROM [test] LIMIT 10");
    }
}
