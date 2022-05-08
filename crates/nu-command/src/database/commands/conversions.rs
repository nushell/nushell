use crate::{database::values::definitions::ConnectionDb, SQLiteDatabase};
use nu_protocol::{ShellError, Value};
use sqlparser::ast::{ObjectName, Statement, TableAlias, TableFactor};

pub fn value_into_table_factor(
    table: Value,
    connection: &ConnectionDb,
    alias: Option<TableAlias>,
) -> Result<TableFactor, ShellError> {
    match table {
        Value::String { val, .. } => {
            let ident = sqlparser::ast::Ident {
                value: val,
                quote_style: None,
            };

            Ok(TableFactor::Table {
                name: ObjectName(vec![ident]),
                alias,
                args: Vec::new(),
                with_hints: Vec::new(),
            })
        }
        Value::CustomValue { span, .. } => {
            let db = SQLiteDatabase::try_from_value(table)?;

            if &db.connection != connection {
                return Err(ShellError::GenericError(
                    "Incompatible connections".into(),
                    "trying to join on table with different connection".into(),
                    Some(span),
                    None,
                    Vec::new(),
                ));
            }

            match db.statement {
                Some(statement) => match statement {
                    Statement::Query(query) => Ok(TableFactor::Derived {
                        lateral: false,
                        subquery: query,
                        alias,
                    }),
                    s => Err(ShellError::GenericError(
                        "Connection doesnt define a query".into(),
                        format!("Expected a connection with query. Got {}", s),
                        Some(span),
                        None,
                        Vec::new(),
                    )),
                },
                None => Err(ShellError::GenericError(
                    "Error creating derived table".into(),
                    "there is no statement defined yet".into(),
                    Some(span),
                    None,
                    Vec::new(),
                )),
            }
        }
        _ => Err(ShellError::UnsupportedInput(
            "String or connection".into(),
            table.span()?,
        )),
    }
}
