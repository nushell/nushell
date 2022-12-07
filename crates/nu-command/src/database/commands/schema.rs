use super::super::SQLiteDatabase;
use crate::database::values::definitions::{db_row::DbRow, db_table::DbTable};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use rusqlite::Connection;
#[derive(Clone)]
pub struct SchemaDb;

impl Command for SchemaDb {
    fn name(&self) -> &str {
        "schema"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Any)
            .output_type(Type::Any)
            .category(Category::Custom("database".into()))
    }

    fn usage(&self) -> &str {
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
        let mut cols = vec![];
        let mut vals = vec![];
        let span = call.head;

        let sqlite_db = SQLiteDatabase::try_from_pipeline(input, span)?;
        let conn = open_sqlite_db_connection(&sqlite_db, span)?;
        let tables = sqlite_db.get_tables(&conn).map_err(|e| {
            ShellError::GenericError(
                "Error reading tables".into(),
                e.to_string(),
                Some(span),
                None,
                Vec::new(),
            )
        })?;

        let mut table_names = vec![];
        let mut table_values = vec![];
        for table in tables {
            let column_info = get_table_columns(&sqlite_db, &conn, &table, span)?;
            let constraint_info = get_table_constraints(&sqlite_db, &conn, &table, span)?;
            let foreign_key_info = get_table_foreign_keys(&sqlite_db, &conn, &table, span)?;
            let index_info = get_table_indexes(&sqlite_db, &conn, &table, span)?;

            let mut cols = vec![];
            let mut vals = vec![];

            cols.push("columns".into());
            vals.push(Value::List {
                vals: column_info,
                span,
            });

            cols.push("constraints".into());
            vals.push(Value::List {
                vals: constraint_info,
                span,
            });

            cols.push("foreign_keys".into());
            vals.push(Value::List {
                vals: foreign_key_info,
                span,
            });

            cols.push("indexes".into());
            vals.push(Value::List {
                vals: index_info,
                span,
            });

            table_names.push(table.name);
            table_values.push(Value::Record { cols, vals, span });
        }

        cols.push("tables".into());
        vals.push(Value::Record {
            cols: table_names,
            vals: table_values,
            span,
        });

        // TODO: add views and triggers

        Ok(PipelineData::Value(
            Value::Record { cols, vals, span },
            None,
        ))
    }
}

fn open_sqlite_db_connection(db: &SQLiteDatabase, span: Span) -> Result<Connection, ShellError> {
    db.open_connection().map_err(|e| {
        ShellError::GenericError(
            "Error opening file".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })
}

fn get_table_columns(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let columns = db.get_columns(conn, table).map_err(|e| {
        ShellError::GenericError(
            "Error getting database columns".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })?;

    // a record of column name = column value
    let mut column_info = vec![];
    for t in columns {
        let mut col_names = vec![];
        let mut col_values = vec![];
        let fields = t.fields();
        let columns = t.columns();
        for (k, v) in fields.iter().zip(columns.iter()) {
            col_names.push(k.clone());
            col_values.push(Value::string(v.clone(), span));
        }
        column_info.push(Value::Record {
            cols: col_names.clone(),
            vals: col_values.clone(),
            span,
        });
    }

    Ok(column_info)
}

fn get_table_constraints(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let constraints = db.get_constraints(conn, table).map_err(|e| {
        ShellError::GenericError(
            "Error getting DB constraints".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })?;
    let mut constraint_info = vec![];
    for constraint in constraints {
        let mut con_cols = vec![];
        let mut con_vals = vec![];
        let fields = constraint.fields();
        let columns = constraint.columns();
        for (k, v) in fields.iter().zip(columns.iter()) {
            con_cols.push(k.clone());
            con_vals.push(Value::string(v.clone(), span));
        }
        constraint_info.push(Value::Record {
            cols: con_cols.clone(),
            vals: con_vals.clone(),
            span,
        });
    }

    Ok(constraint_info)
}

fn get_table_foreign_keys(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let foreign_keys = db.get_foreign_keys(conn, table).map_err(|e| {
        ShellError::GenericError(
            "Error getting DB Foreign Keys".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })?;
    let mut foreign_key_info = vec![];
    for fk in foreign_keys {
        let mut fk_cols = vec![];
        let mut fk_vals = vec![];
        let fields = fk.fields();
        let columns = fk.columns();
        for (k, v) in fields.iter().zip(columns.iter()) {
            fk_cols.push(k.clone());
            fk_vals.push(Value::string(v.clone(), span));
        }
        foreign_key_info.push(Value::Record {
            cols: fk_cols.clone(),
            vals: fk_vals.clone(),
            span,
        });
    }

    Ok(foreign_key_info)
}

fn get_table_indexes(
    db: &SQLiteDatabase,
    conn: &Connection,
    table: &DbTable,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let indexes = db.get_indexes(conn, table).map_err(|e| {
        ShellError::GenericError(
            "Error getting DB Indexes".into(),
            e.to_string(),
            Some(span),
            None,
            Vec::new(),
        )
    })?;
    let mut index_info = vec![];
    for index in indexes {
        let mut idx_cols = vec![];
        let mut idx_vals = vec![];
        let fields = index.fields();
        let columns = index.columns();
        for (k, v) in fields.iter().zip(columns.iter()) {
            idx_cols.push(k.clone());
            idx_vals.push(Value::string(v.clone(), span));
        }
        index_info.push(Value::Record {
            cols: idx_cols.clone(),
            vals: idx_vals.clone(),
            span,
        });
    }

    Ok(index_info)
}
