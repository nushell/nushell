use crate::database::values::sqlite::{open_sqlite_db, values_to_sql};
use nu_engine::command_prelude::*;

use itertools::Itertools;
use nu_protocol::Signals;
use std::path::Path;

pub const DEFAULT_TABLE_NAME: &str = "main";

#[derive(Clone)]
pub struct IntoSqliteDb;

impl Command for IntoSqliteDb {
    fn name(&self) -> &str {
        "into sqlite"
    }

    fn signature(&self) -> Signature {
        Signature::build("into sqlite")
            .category(Category::Conversions)
            .input_output_types(vec![
                (Type::table(), Type::Nothing),
                (Type::record(), Type::Nothing),
            ])
            .allow_variants_without_examples(true)
            .required(
                "file-name",
                SyntaxShape::String,
                "Specify the filename to save the database to.",
            )
            .named(
                "table-name",
                SyntaxShape::String,
                "Specify table name to store the data in",
                Some('t'),
            )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        operate(engine_state, stack, call, input)
    }

    fn description(&self) -> &str {
        "Convert table into a SQLite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "database"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Convert ls entries into a SQLite database with 'main' as the table name",
                example: "ls | into sqlite my_ls.db",
                result: None,
            },
            Example {
                description: "Convert ls entries into a SQLite database with 'my_table' as the table name",
                example: "ls | into sqlite my_ls.db -t my_table",
                result: None,
            },
            Example {
                description: "Convert table literal into a SQLite database with 'main' as the table name",
                example: "[[name]; [-----] [someone] [=====] [somename] ['(((((']] | into sqlite filename.db",
                result: None,
            },
            Example {
                description: "Insert a single record into a SQLite database",
                example: "{ foo: bar, baz: quux } | into sqlite filename.db",
                result: None,
            },
        ]
    }
}

struct Table {
    conn: rusqlite::Connection,
    table_name: String,
}

impl Table {
    pub fn new(
        db_path: &Spanned<String>,
        table_name: Option<Spanned<String>>,
    ) -> Result<Self, nu_protocol::ShellError> {
        let table_name = if let Some(table_name) = table_name {
            table_name.item
        } else {
            DEFAULT_TABLE_NAME.to_string()
        };

        // create the sqlite database table
        let conn = open_sqlite_db(Path::new(&db_path.item), db_path.span)?;

        Ok(Self { conn, table_name })
    }

    pub fn name(&self) -> &String {
        &self.table_name
    }

    fn try_init(
        &mut self,
        record: &Record,
    ) -> Result<rusqlite::Transaction, nu_protocol::ShellError> {
        let first_row_null = record.values().any(Value::is_nothing);
        let columns = get_columns_with_sqlite_types(record)?;

        let table_exists_query = format!(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{}';",
            self.name(),
        );

        let table_count: u64 = self
            .conn
            .query_row(&table_exists_query, [], |row| row.get(0))
            .map_err(|err| ShellError::GenericError {
                error: format!("{err:#?}"),
                msg: format!("{err:#?}"),
                span: None,
                help: None,
                inner: Vec::new(),
            })?;

        if table_count == 0 {
            if first_row_null {
                eprintln!(
                    "Warning: The first row contains a null value, which has an \
unknown SQL type. Null values will be assumed to be TEXT columns. \
If this is undesirable, you can create the table first with your desired schema."
                );
            }

            // create a string for sql table creation
            let create_statement = format!(
                "CREATE TABLE [{}] ({})",
                self.table_name,
                columns
                    .into_iter()
                    .map(|(col_name, sql_type)| format!("{col_name} {sql_type}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            // execute the statement
            self.conn
                .execute(&create_statement, [])
                .map_err(|err| ShellError::GenericError {
                    error: "Failed to create table".into(),
                    msg: err.to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                })?;
        }

        self.conn
            .transaction()
            .map_err(|err| ShellError::GenericError {
                error: "Failed to open transaction".into(),
                msg: err.to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            })
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let file_name: Spanned<String> = call.req(engine_state, stack, 0)?;
    let table_name: Option<Spanned<String>> = call.get_flag(engine_state, stack, "table-name")?;
    let table = Table::new(&file_name, table_name)?;
    Ok(action(input, table, span, engine_state.signals())?.into_pipeline_data())
}

fn action(
    input: PipelineData,
    table: Table,
    span: Span,
    signals: &Signals,
) -> Result<Value, ShellError> {
    match input {
        PipelineData::ListStream(stream, _) => {
            insert_in_transaction(stream.into_iter(), span, table, signals)
        }
        PipelineData::Value(value @ Value::List { .. }, _) => {
            let span = value.span();
            let vals = value
                .into_list()
                .expect("Value matched as list above, but is not a list");
            insert_in_transaction(vals.into_iter(), span, table, signals)
        }
        PipelineData::Value(val, _) => {
            insert_in_transaction(std::iter::once(val), span, table, signals)
        }
        _ => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "list".into(),
            wrong_type: "".into(),
            dst_span: span,
            src_span: span,
        }),
    }
}

fn insert_in_transaction(
    stream: impl Iterator<Item = Value>,
    span: Span,
    mut table: Table,
    signals: &Signals,
) -> Result<Value, ShellError> {
    let mut stream = stream.peekable();
    let first_val = match stream.peek() {
        None => return Ok(Value::nothing(span)),
        Some(val) => val.as_record()?.clone(),
    };

    if first_val.is_empty() {
        Err(ShellError::GenericError {
            error: "Failed to create table".into(),
            msg: "Cannot create table without columns".to_string(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;
    }

    let table_name = table.name().clone();
    let tx = table.try_init(&first_val)?;

    for stream_value in stream {
        if let Err(err) = signals.check(span) {
            tx.rollback().map_err(|e| ShellError::GenericError {
                error: "Failed to rollback SQLite transaction".into(),
                msg: e.to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            })?;
            return Err(err);
        }

        let val = stream_value.as_record()?;

        let insert_statement = format!(
            "INSERT INTO [{}] ({}) VALUES ({})",
            table_name,
            Itertools::intersperse(val.columns().map(|c| format!("`{c}`")), ", ".to_string())
                .collect::<String>(),
            Itertools::intersperse(itertools::repeat_n("?", val.len()), ", ").collect::<String>(),
        );

        let mut insert_statement =
            tx.prepare(&insert_statement)
                .map_err(|e| ShellError::GenericError {
                    error: "Failed to prepare SQLite statement".into(),
                    msg: e.to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                })?;

        let result = insert_value(stream_value, &mut insert_statement);

        insert_statement
            .finalize()
            .map_err(|e| ShellError::GenericError {
                error: "Failed to finalize SQLite prepared statement".into(),
                msg: e.to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            })?;

        result?
    }

    tx.commit().map_err(|e| ShellError::GenericError {
        error: "Failed to commit SQLite transaction".into(),
        msg: e.to_string(),
        span: None,
        help: None,
        inner: Vec::new(),
    })?;

    Ok(Value::nothing(span))
}

fn insert_value(
    stream_value: Value,
    insert_statement: &mut rusqlite::Statement<'_>,
) -> Result<(), ShellError> {
    match stream_value {
        // map each column value into its SQL representation
        Value::Record { val, .. } => {
            let sql_vals = values_to_sql(val.values().cloned())?;

            insert_statement
                .execute(rusqlite::params_from_iter(sql_vals))
                .map_err(|e| ShellError::GenericError {
                    error: "Failed to execute SQLite statement".into(),
                    msg: e.to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                })?;

            Ok(())
        }
        val => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "record".into(),
            wrong_type: val.get_type().to_string(),
            dst_span: Span::unknown(),
            src_span: val.span(),
        }),
    }
}

// Each value stored in an SQLite database (or manipulated by the database engine) has one of the following storage classes:
// NULL. The value is a NULL value.
// INTEGER. The value is a signed integer, stored in 0, 1, 2, 3, 4, 6, or 8 bytes depending on the magnitude of the value.
// REAL. The value is a floating point value, stored as an 8-byte IEEE floating point number.
// TEXT. The value is a text string, stored using the database encoding (UTF-8, UTF-16BE or UTF-16LE).
// BLOB. The value is a blob of data, stored exactly as it was input.
fn nu_value_to_sqlite_type(val: &Value) -> Result<&'static str, ShellError> {
    match val.get_type() {
        Type::String => Ok("TEXT"),
        Type::Int => Ok("INTEGER"),
        Type::Float => Ok("REAL"),
        Type::Number => Ok("REAL"),
        Type::Binary => Ok("BLOB"),
        Type::Bool => Ok("BOOLEAN"),
        Type::Date => Ok("DATETIME"),
        Type::Duration => Ok("BIGINT"),
        Type::Filesize => Ok("INTEGER"),

        // [NOTE] On null values, we just assume TEXT. This could end up
        // creating a table where the column type is wrong in the table schema.
        // This means the table could end up with the wrong schema.
        Type::Nothing => Ok("TEXT"),

        // intentionally enumerated so that any future types get handled
        Type::Any
        | Type::Block
        | Type::CellPath
        | Type::Closure
        | Type::Custom(_)
        | Type::Error
        | Type::List(_)
        | Type::Range
        | Type::Record(_)
        | Type::Glob
        | Type::Table(_) => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: "sql".into(),
            wrong_type: val.get_type().to_string(),
            dst_span: Span::unknown(),
            src_span: val.span(),
        }),
    }
}

fn get_columns_with_sqlite_types(
    record: &Record,
) -> Result<Vec<(String, &'static str)>, ShellError> {
    let mut columns: Vec<(String, &'static str)> = vec![];

    for (c, v) in record {
        if !columns
            .iter()
            .map(|name| (format!("`{}`", name.0), name.1))
            .any(|(name, _)| name == *c)
        {
            columns.push((format!("`{c}`"), nu_value_to_sqlite_type(v)?));
        }
    }

    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    // use super::{action, IntoSqliteDb};
    // use nu_protocol::Type::Error;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(IntoSqliteDb {})
    }
}
