use super::super::SQLiteDatabase;
use crate::database::values::definitions::{db_row::DbRow, db_table::DbTable};
use nu_engine::command_prelude::*;
use rusqlite::Connection;

#[derive(Clone)]
pub struct SchemaDb;

impl Command for SchemaDb {
    fn name(&self) -> &str {
        "schema"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Any, Type::Any)])
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Show the schema of a SQLite database."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show the schema of a SQLite database",
            example: r#"open foo.db | schema"#,
            result: None,
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "info", "SQLite"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let sqlite_db = SQLiteDatabase::try_from_pipeline(input, span)?;
        let conn = open_sqlite_db_connection(&sqlite_db, span)?;
        let tables = sqlite_db
            .get_tables(&conn)
            .map_err(|e| ShellError::GenericError {
                error: "Error reading tables".into(),
                msg: e.to_string(),
                span: Some(span),
                help: None,
                inner: vec![],
            })?;

        let mut tables_record = Record::new();
        for table in tables {
            let column_info = get_table_columns(&sqlite_db, &conn, &table, span)?;
            let constraint_info = get_table_constraints(&sqlite_db, &conn, &table, span)?;
            let foreign_key_info = get_table_foreign_keys(&sqlite_db, &conn, &table, span)?;
            let index_info = get_table_indexes(&sqlite_db, &conn, &table, span)?;

            tables_record.push(
                table.name,
                Value::record(
                    record! {
                        "columns" => Value::list(column_info, span),
                        "constraints" => Value::list(constraint_info, span),
                        "foreign_keys" => Value::list(foreign_key_info, span),
                        "indexes" => Value::list(index_info, span),
                    },
                    span,
                ),
            );
        }

        let record = record! { "tables" => Value::record(tables_record, span) };

        // TODO: add views and triggers

        Ok(PipelineData::value(Value::record(record, span), None))
    }
}

fn open_sqlite_db_connection(db: &SQLiteDatabase, span: Span) -> Result<Connection, ShellError> {
    db.open_connection().map_err(|e| ShellError::GenericError {
        error: "Error opening file".into(),
        msg: e.to_string(),
        span: Some(span),
        help: None,
        inner: vec![],
    })
}

fn get_table_columns(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let columns = db
        .get_columns(conn, table)
        .map_err(|e| ShellError::GenericError {
            error: "Error getting database columns".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;

    // a record of column name = column value
    let mut column_info = vec![];
    for t in columns {
        column_info.push(Value::record(
            t.fields()
                .into_iter()
                .zip(t.columns())
                .map(|(k, v)| (k, Value::string(v, span)))
                .collect(),
            span,
        ));
    }

    Ok(column_info)
}

fn get_table_constraints(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let constraints = db
        .get_constraints(conn, table)
        .map_err(|e| ShellError::GenericError {
            error: "Error getting DB constraints".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;
    let mut constraint_info = vec![];
    for constraint in constraints {
        constraint_info.push(Value::record(
            constraint
                .fields()
                .into_iter()
                .zip(constraint.columns())
                .map(|(k, v)| (k, Value::string(v, span)))
                .collect(),
            span,
        ));
    }

    Ok(constraint_info)
}

fn get_table_foreign_keys(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let foreign_keys = db
        .get_foreign_keys(conn, table)
        .map_err(|e| ShellError::GenericError {
            error: "Error getting DB Foreign Keys".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;
    let mut foreign_key_info = vec![];
    for fk in foreign_keys {
        foreign_key_info.push(Value::record(
            fk.fields()
                .into_iter()
                .zip(fk.columns())
                .map(|(k, v)| (k, Value::string(v, span)))
                .collect(),
            span,
        ));
    }

    Ok(foreign_key_info)
}

fn get_table_indexes(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let indexes = db
        .get_indexes(conn, table)
        .map_err(|e| ShellError::GenericError {
            error: "Error getting DB Indexes".into(),
            msg: e.to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;
    let mut index_info = vec![];
    for index in indexes {
        index_info.push(Value::record(
            index
                .fields()
                .into_iter()
                .zip(index.columns())
                .map(|(k, v)| (k, Value::string(v, span)))
                .collect(),
            span,
        ));
    }

    Ok(index_info)
}
