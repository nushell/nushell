use crate::database::values::sqlite::open_sqlite_db;
use itertools::Itertools;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Config, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};
use std::iter;
use std::path::Path;

#[derive(Clone)]
pub struct IntoSqliteDb;

impl Command for IntoSqliteDb {
    fn name(&self) -> &str {
        "into sqlite"
    }

    fn signature(&self) -> Signature {
        Signature::build("into sqlite")
            .required(
                "file_name",
                SyntaxShape::String,
                "Specify the filename to save the database to",
            )
            .named(
                "table_name",
                SyntaxShape::String,
                "Specify table name to store the data in",
                Some('t'),
            )
            .category(Category::Conversions)
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

    fn usage(&self) -> &str {
        "Convert table into a SQLite database"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "database"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
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
            description: "Convert a variety of values in table literal form into a SQLite database",
            example: "[one 2 5.2 six true 100mib 25sec] | into sqlite variety.db",
            result: None,
        }]
    }
}

fn operate(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let config = engine_state.get_config();
    let file_name: Spanned<String> = call.req(engine_state, stack, 0)?;
    let table_name: Option<Spanned<String>> = call.get_flag(engine_state, stack, "table_name")?;

    // collect the input into a value
    let table_entries = input.into_value(span);

    match action(&table_entries, table_name, file_name, config, span) {
        Ok(val) => Ok(val.into_pipeline_data()),
        Err(e) => Err(e),
    }
}

fn action(
    input: &Value,
    table: Option<Spanned<String>>,
    file: Spanned<String>,
    config: &Config,
    span: Span,
) -> Result<Value, ShellError> {
    let table_name = if let Some(table_name) = table {
        table_name.item
    } else {
        "main".to_string()
    };

    match input {
        Value::List { vals, span } => {
            // find the column names, and sqlite data types
            let columns = get_columns_with_sqlite_types(vals);

            let table_columns_creation = columns
                .iter()
                .map(|(name, sql_type)| format!("{} {}", name, sql_type))
                .join(",");

            // get the values
            let table_values = vals
                .iter()
                .map(|list_value| {
                    format!(
                        "({})",
                        match list_value {
                            Value::Record {
                                cols: _,
                                vals,
                                span: _,
                            } => {
                                vals.iter()
                                    .map(|rec_val| {
                                        format!(
                                            "'{}'",
                                            nu_value_to_string(rec_val.clone(), "", config)
                                        )
                                    })
                                    .join(",")
                            }
                            // Number formats so keep them without quotes
                            Value::Int { val: _, span: _ }
                            | Value::Float { val: _, span: _ }
                            | Value::Filesize { val: _, span: _ }
                            | Value::Duration { val: _, span: _ } =>
                                nu_value_to_string(list_value.clone(), "", config),
                            _ =>
                            // String formats so add quotes around them
                                format!("'{}'", nu_value_to_string(list_value.clone(), "", config)),
                        }
                    )
                })
                .join(",");

            // create the sqlite database table
            let conn = open_sqlite_db(Path::new(&file.item), file.span)?;

            // create a string for sql table creation
            let create_statement = format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                table_name, table_columns_creation
            );

            // prepare the string as a sqlite statement
            let mut stmt = conn.prepare(&create_statement).map_err(|e| {
                ShellError::GenericError(
                    "Failed to prepare SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // execute the statement
            stmt.execute([]).map_err(|e| {
                ShellError::GenericError(
                    "Failed to execute SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // use normal sql to create the table
            // insert into table_name
            // values
            // ('xx', 'yy', 'zz'),
            // ('aa', 'bb', 'cc'),
            // ('dd', 'ee', 'ff')

            // create the string for inserting data into the table
            let insert_statement = format!("INSERT INTO {} VALUES {}", table_name, table_values);

            // prepare the string as a sqlite statement
            let mut stmt = conn.prepare(&insert_statement).map_err(|e| {
                ShellError::GenericError(
                    "Failed to prepare SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // execute the statement
            stmt.execute([]).map_err(|e| {
                ShellError::GenericError(
                    "Failed to execute SQLite statement".into(),
                    e.to_string(),
                    Some(file.span),
                    None,
                    Vec::new(),
                )
            })?;

            // and we're done
            Ok(Value::Nothing { span: *span })
        }
        _ => Err(ShellError::UnsupportedInput(
            format!(
                "Expected a list but instead received a {}",
                input.get_type()
            ),
            span,
        )),
    }
}

// This is taken from to text local_into_string but tweaks it a bit so that certain formatting does not happen
fn nu_value_to_string(value: Value, separator: &str, config: &Config) -> String {
    match value {
        Value::Bool { val, .. } => val.to_string(),
        Value::Int { val, .. } => val.to_string(),
        Value::Float { val, .. } => val.to_string(),
        Value::Filesize { val, .. } => val.to_string(),
        Value::Duration { val, .. } => val.to_string(),
        Value::Date { val, .. } => val.to_string(),
        Value::Range { val, .. } => {
            format!(
                "{}..{}",
                nu_value_to_string(val.from, ", ", config),
                nu_value_to_string(val.to, ", ", config)
            )
        }
        Value::String { val, .. } => {
            // don't store ansi escape sequences in the database
            // escape single quotes
            nu_utils::strip_ansi_unlikely(&val).replace('\'', "''")
        }
        Value::List { vals: val, .. } => val
            .iter()
            .map(|x| nu_value_to_string(x.clone(), ", ", config))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Record { cols, vals, .. } => cols
            .iter()
            .zip(vals.iter())
            .map(|(x, y)| format!("{}: {}", x, nu_value_to_string(y.clone(), ", ", config)))
            .collect::<Vec<_>>()
            .join(separator),
        Value::Block { val, .. } => format!("<Block {}>", val),
        Value::Closure { val, .. } => format!("<Closure {}>", val),
        Value::Nothing { .. } => String::new(),
        Value::Error { error } => format!("{:?}", error),
        Value::Binary { val, .. } => format!("{:?}", val),
        Value::CellPath { val, .. } => val.into_string(),
        Value::CustomValue { val, .. } => val.value_string(),
    }
}

// Each value stored in an SQLite database (or manipulated by the database engine) has one of the following storage classes:
// NULL. The value is a NULL value.
// INTEGER. The value is a signed integer, stored in 0, 1, 2, 3, 4, 6, or 8 bytes depending on the magnitude of the value.
// REAL. The value is a floating point value, stored as an 8-byte IEEE floating point number.
// TEXT. The value is a text string, stored using the database encoding (UTF-8, UTF-16BE or UTF-16LE).
// BLOB. The value is a blob of data, stored exactly as it was input.
fn nu_type_to_sqlite_type(nu_type: Type) -> &'static str {
    match nu_type {
        Type::Int => "INTEGER",
        Type::Float => "REAL",
        Type::String => "TEXT",
        Type::Bool => "TEXT",
        Type::Nothing => "NULL",
        Type::Filesize => "INTEGER",
        Type::Date => "TEXT",
        _ => "TEXT",
    }
}

fn get_columns_with_sqlite_types(input: &[Value]) -> Vec<(String, String)> {
    let mut columns: Vec<(String, String)> = vec![];
    let mut added = false;

    for item in input {
        // let sqlite_type = nu_type_to_sqlite_type(item.get_type());
        // eprintln!(
        //     "item_type: {:?}, sqlite_type: {:?}",
        //     item.get_type(),
        //     sqlite_type
        // );

        if let Value::Record { cols, vals, .. } = item {
            for (c, v) in iter::zip(cols, vals) {
                if !columns.iter().any(|(name, _)| name == c) {
                    columns.push((
                        c.to_string(),
                        nu_type_to_sqlite_type(v.get_type()).to_string(),
                    ));
                }
            }
        } else {
            // force every other type to a string
            if !added {
                columns.push(("value".to_string(), "TEXT".to_string()));
                added = true;
            }
        }
    }

    columns
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
