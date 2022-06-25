use crate::database::values::definitions::ConnectionDb;

use super::{super::SQLiteDatabase, conversions::value_into_table_factor};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Type, Value,
};
use sqlparser::ast::{Ident, Query, Select, SetExpr, Statement, TableAlias, TableWithJoins};

#[derive(Clone)]
pub struct FromDb;

impl Command for FromDb {
    fn name(&self) -> &str {
        "from"
    }

    fn usage(&self) -> &str {
        "Select section from query statement for a DB"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "select",
                SyntaxShape::Any,
                "table of derived table to select from",
            )
            .named(
                "as",
                SyntaxShape::String,
                "Alias for the selected table",
                Some('a'),
            )
            .input_type(Type::Custom("database".into()))
            .output_type(Type::Custom("database".into()))
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "from"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Selects a table from database",
            example: "open db.mysql | into db | from table_a | describe",
            result: Some(Value::Record {
                cols: vec!["connection".into(), "query".into()],
                vals: vec![
                    Value::String {
                        val: "db.mysql".into(),
                        span: Span::test_data(),
                    },
                    Value::String {
                        val: "SELECT  FROM table_a".into(),
                        span: Span::test_data(),
                    },
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.statement = match db.statement {
            None => Some(create_statement(&db.connection, engine_state, stack, call)?),
            Some(statement) => Some(modify_statement(
                &db.connection,
                statement,
                engine_state,
                stack,
                call,
            )?),
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn create_statement(
    connection: &ConnectionDb,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Statement, ShellError> {
    let query = Query {
        with: None,
        body: SetExpr::Select(Box::new(create_select(
            connection,
            engine_state,
            stack,
            call,
        )?)),
        order_by: Vec::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    };

    Ok(Statement::Query(Box::new(query)))
}

fn modify_statement(
    connection: &ConnectionDb,
    mut statement: Statement,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Statement, ShellError> {
    match statement {
        Statement::Query(ref mut query) => {
            match query.body {
                SetExpr::Select(ref mut select) => {
                    let table = create_table(connection, engine_state, stack, call)?;
                    select.from.push(table);
                }
                _ => {
                    query.as_mut().body = SetExpr::Select(Box::new(create_select(
                        connection,
                        engine_state,
                        stack,
                        call,
                    )?));
                }
            };

            Ok(statement)
        }
        s => Err(ShellError::GenericError(
            "Connection doesnt define a query".into(),
            format!("Expected a connection with query. Got {}", s),
            Some(call.head),
            None,
            Vec::new(),
        )),
    }
}

fn create_select(
    connection: &ConnectionDb,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Select, ShellError> {
    Ok(Select {
        distinct: false,
        top: None,
        projection: Vec::new(),
        into: None,
        from: vec![create_table(connection, engine_state, stack, call)?],
        lateral_views: Vec::new(),
        selection: None,
        group_by: Vec::new(),
        cluster_by: Vec::new(),
        distribute_by: Vec::new(),
        sort_by: Vec::new(),
        having: None,
    })
}

fn create_table(
    connection: &ConnectionDb,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<TableWithJoins, ShellError> {
    let alias = call
        .get_flag::<String>(engine_state, stack, "as")?
        .map(|alias| TableAlias {
            name: Ident {
                value: alias,
                quote_style: None,
            },
            columns: Vec::new(),
        });

    let select_table: Value = call.req(engine_state, stack, 0)?;
    let table_factor = value_into_table_factor(select_table, connection, alias)?;

    let table = TableWithJoins {
        relation: table_factor,
        joins: Vec::new(),
    };

    Ok(table)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::test_database::test_database;

    #[test]
    fn test_examples() {
        test_database(vec![Box::new(FromDb {})])
    }
}
