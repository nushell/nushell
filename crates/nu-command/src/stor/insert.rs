use crate::database::{MEMORY_DB, SQLiteDatabase, values_to_sql};
use nu_engine::command_prelude::*;
use nu_protocol::Signals;
use rusqlite::params_from_iter;

#[derive(Clone)]
pub struct StorInsert;

impl Command for StorInsert {
    fn name(&self) -> &str {
        "stor insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("stor insert")
            .input_output_types(vec![
                (Type::Nothing, Type::table()),
                (Type::record(), Type::table()),
                (Type::table(), Type::table()),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::table()),
            ])
            .required_named(
                "table-name",
                SyntaxShape::String,
                "name of the table you want to insert into",
                Some('t'),
            )
            .named(
                "data-record",
                SyntaxShape::Record(vec![]),
                "a record of column names and column values to insert into the specified table",
                Some('d'),
            )
            .allow_variants_without_examples(true)
            .category(Category::Database)
    }

    fn description(&self) -> &str {
        "Insert information into a specified table in the in-memory sqlite database."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["sqlite", "storing", "table", "saving"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Insert data in the in-memory sqlite database using a data-record of column-name and column-value pairs",
                example: "stor insert --table-name nudb --data-record {bool1: true, int1: 5, float1: 1.1, str1: fdncred, datetime1: 2023-04-17}",
                result: None,
            },
            Example {
                description: "Insert data through pipeline input as a record of column-name and column-value pairs",
                example: "{bool1: true, int1: 5, float1: 1.1, str1: fdncred, datetime1: 2023-04-17} | stor insert --table-name nudb",
                result: None,
            },
            Example {
                description: "Insert data through pipeline input as a table literal",
                example: "[[bool1 int1 float1]; [true 5 1.1], [false 8 3.14]] | stor insert --table-name nudb",
                result: None,
            },
            Example {
                description: "Insert ls entries",
                example: "ls | stor insert --table-name files",
                result: None,
            },
            Example {
                description: "Insert nu records as json data",
                example: "ls -l | each {{file: $in.name, metadata: ($in | reject name)}} | stor insert --table-name files_with_md",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let table_name: Option<String> = call.get_flag(engine_state, stack, "table-name")?;
        let data_record: Option<Record> = call.get_flag(engine_state, stack, "data-record")?;
        // let config = stack.get_config(engine_state);
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        let records = handle(span, data_record, input)?;

        for record in records {
            process(engine_state, table_name.clone(), span, &db, record)?;
        }

        Ok(Value::custom(db, span).into_pipeline_data())
    }
}

fn handle(
    span: Span,
    data_record: Option<Record>,
    input: PipelineData,
) -> Result<Vec<Record>, ShellError> {
    // Check for conflicting use of both pipeline input and flag
    if let Some(record) = data_record {
        if !matches!(input, PipelineData::Empty) {
            return Err(ShellError::GenericError {
                error: "Pipeline and Flag both being used".into(),
                msg: "Use either pipeline input or '--data-record' parameter".into(),
                span: Some(span),
                help: None,
                inner: vec![],
            });
        }
        return Ok(vec![record]);
    }

    // Handle the input types
    let values = match input {
        PipelineData::Empty => {
            return Err(ShellError::MissingParameter {
                param_name: "requires a table or a record".into(),
                span,
            });
        }
        PipelineData::ListStream(stream, ..) => stream.into_iter().collect::<Vec<_>>(),
        PipelineData::Value(Value::List { vals, .. }, ..) => vals,
        PipelineData::Value(val, ..) => vec![val],
        _ => {
            return Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "list or record".into(),
                wrong_type: "".into(),
                dst_span: span,
                src_span: span,
            });
        }
    };

    values
        .into_iter()
        .map(|val| match val {
            Value::Record { val, .. } => Ok(val.into_owned()),
            other => Err(ShellError::OnlySupportsThisInputType {
                exp_input_type: "record".into(),
                wrong_type: other.get_type().to_string(),
                dst_span: Span::unknown(),
                src_span: other.span(),
            }),
        })
        .collect()
}

fn process(
    engine_state: &EngineState,
    table_name: Option<String>,
    span: Span,
    db: &SQLiteDatabase,
    record: Record,
) -> Result<(), ShellError> {
    if table_name.is_none() {
        return Err(ShellError::MissingParameter {
            param_name: "requires at table name".into(),
            span,
        });
    }
    let new_table_name = table_name.unwrap_or("table".into());

    let mut create_stmt = format!("INSERT INTO {new_table_name} (");
    let mut column_placeholders: Vec<String> = Vec::new();

    let cols = record.columns();
    cols.for_each(|col| {
        column_placeholders.push(col.to_string());
    });

    create_stmt.push_str(&column_placeholders.join(", "));

    // Values are set as placeholders.
    create_stmt.push_str(") VALUES (");
    let mut value_placeholders: Vec<String> = Vec::new();
    for (index, _) in record.columns().enumerate() {
        value_placeholders.push(format!("?{}", index + 1));
    }
    create_stmt.push_str(&value_placeholders.join(", "));
    create_stmt.push(')');

    // dbg!(&create_stmt);

    // Get the params from the passed values
    let params = values_to_sql(engine_state, record.values().cloned(), span)?;

    if let Ok(conn) = db.open_connection() {
        conn.execute(&create_stmt, params_from_iter(params))
            .map_err(|err| ShellError::GenericError {
                error: "Failed to insert using the SQLite connection in memory from insert.rs."
                    .into(),
                msg: err.to_string(),
                span: Some(Span::test_data()),
                help: None,
                inner: vec![],
            })?;
    };
    // dbg!(db.clone());
    Ok(())
}

#[cfg(test)]
mod test {
    use chrono::DateTime;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(StorInsert {})
    }

    #[test]
    fn test_process_with_simple_parameters() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let create_stmt = "CREATE TABLE test_process_with_simple_parameters (
            int_column INTEGER,
            real_column REAL,
            str_column VARCHAR(255),
            bool_column BOOLEAN,
            date_column DATETIME DEFAULT(STRFTIME('%Y-%m-%d %H:%M:%f', 'NOW'))
        )";

        let conn = db
            .open_connection()
            .expect("Test was unable to open connection.");
        conn.execute(create_stmt, [])
            .expect("Failed to create table as part of test.");
        let table_name = Some("test_process_with_simple_parameters".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert("int_column".to_string(), Value::test_int(42));
        columns.insert("real_column".to_string(), Value::test_float(3.1));
        columns.insert(
            "str_column".to_string(),
            Value::test_string("SimpleString".to_string()),
        );
        columns.insert("bool_column".to_string(), Value::test_bool(true));
        columns.insert(
            "date_column".to_string(),
            Value::test_date(
                DateTime::parse_from_str("2021-12-30 00:00:00 +0000", "%Y-%m-%d %H:%M:%S %z")
                    .expect("Date string should parse."),
            ),
        );

        let result = process(&EngineState::new(), table_name, span, &db, columns);

        assert!(result.is_ok());
    }

    #[test]
    fn test_process_string_with_space() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let create_stmt = "CREATE TABLE test_process_string_with_space (
            str_column VARCHAR(255)
        )";

        let conn = db
            .open_connection()
            .expect("Test was unable to open connection.");
        conn.execute(create_stmt, [])
            .expect("Failed to create table as part of test.");
        let table_name = Some("test_process_string_with_space".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "str_column".to_string(),
            Value::test_string("String With Spaces".to_string()),
        );

        let result = process(&EngineState::new(), table_name, span, &db, columns);

        assert!(result.is_ok());
    }

    #[test]
    fn test_no_errors_when_string_too_long() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let create_stmt = "CREATE TABLE test_errors_when_string_too_long (
            str_column VARCHAR(8)
        )";

        let conn = db
            .open_connection()
            .expect("Test was unable to open connection.");
        conn.execute(create_stmt, [])
            .expect("Failed to create table as part of test.");
        let table_name = Some("test_errors_when_string_too_long".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "str_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(&EngineState::new(), table_name, span, &db, columns);
        // SQLite uses dynamic typing, making any length acceptable for a varchar column
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_errors_when_param_is_wrong_type() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let create_stmt = "CREATE TABLE test_errors_when_param_is_wrong_type (
            int_column INT
        )";

        let conn = db
            .open_connection()
            .expect("Test was unable to open connection.");
        conn.execute(create_stmt, [])
            .expect("Failed to create table as part of test.");
        let table_name = Some("test_errors_when_param_is_wrong_type".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "int_column".to_string(),
            Value::test_string("ThisIsTheWrongType".to_string()),
        );

        let result = process(&EngineState::new(), table_name, span, &db, columns);
        // SQLite uses dynamic typing, making any type acceptable for a column
        assert!(result.is_ok());
    }

    #[test]
    fn test_errors_when_column_doesnt_exist() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));
        let create_stmt = "CREATE TABLE test_errors_when_column_doesnt_exist (
            int_column INT
        )";

        let conn = db
            .open_connection()
            .expect("Test was unable to open connection.");
        conn.execute(create_stmt, [])
            .expect("Failed to create table as part of test.");
        let table_name = Some("test_errors_when_column_doesnt_exist".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "not_a_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(&EngineState::new(), table_name, span, &db, columns);

        assert!(result.is_err());
    }

    #[test]
    fn test_errors_when_table_doesnt_exist() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        let table_name = Some("test_errors_when_table_doesnt_exist".to_string());
        let span = Span::unknown();
        let mut columns = Record::new();
        columns.insert(
            "str_column".to_string(),
            Value::test_string("ThisIsALongString".to_string()),
        );

        let result = process(&EngineState::new(), table_name, span, &db, columns);

        assert!(result.is_err());
    }

    #[test]
    fn test_insert_json() {
        let db = Box::new(SQLiteDatabase::new(
            std::path::Path::new(MEMORY_DB),
            Signals::empty(),
        ));

        let create_stmt = "CREATE TABLE test_insert_json (
            json_field JSON,
            jsonb_field JSONB 
        )";

        let conn = db
            .open_connection()
            .expect("Test was unable to open connection.");
        conn.execute(create_stmt, [])
            .expect("Failed to create table as part of test.");

        let mut record = Record::new();
        record.insert("x", Value::test_int(89));
        record.insert("y", Value::test_int(12));
        record.insert(
            "z",
            Value::test_list(vec![
                Value::test_string("hello"),
                Value::test_string("goodbye"),
            ]),
        );

        let mut row = Record::new();
        row.insert("json_field", Value::test_record(record.clone()));
        row.insert("jsonb_field", Value::test_record(record));

        let result = process(
            &EngineState::new(),
            Some("test_insert_json".to_owned()),
            Span::unknown(),
            &db,
            row,
        );

        assert!(result.is_ok());
    }
}
