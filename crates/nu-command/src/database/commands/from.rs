use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
};
use sqlparser::ast::{
    Ident, ObjectName, Query, Select, SetExpr, Statement, TableFactor, TableWithJoins,
};

#[derive(Clone)]
pub struct FromDb;

impl Command for FromDb {
    fn name(&self) -> &str {
        "db from"
    }

    fn usage(&self) -> &str {
        "Select section from query statement for a DB"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "select",
                SyntaxShape::String,
                "Name of table to select from",
            )
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "from"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Selects table from database",
            example: "db open db.mysql | db from table_a",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let table: String = call.req(engine_state, stack, 0)?;

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.statement = match db.statement {
            None => Some(create_statement(table)),
            Some(statement) => Some(modify_statement(statement, table, call.head)?),
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn create_statement(table: String) -> Statement {
    let query = Query {
        with: None,
        body: SetExpr::Select(Box::new(create_select(table))),
        order_by: Vec::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    };

    Statement::Query(Box::new(query))
}

fn modify_statement(
    mut statement: Statement,
    table: String,
    span: Span,
) -> Result<Statement, ShellError> {
    match statement {
        Statement::Query(ref mut query) => {
            match query.body {
                SetExpr::Select(ref mut select) => select.as_mut().from = create_from(table),
                _ => {
                    query.as_mut().body = SetExpr::Select(Box::new(create_select(table)));
                }
            };

            Ok(statement)
        }
        s => Err(ShellError::GenericError(
            "Connection doesnt define a statement".into(),
            format!("Expected a connection with query. Got {}", s),
            Some(span),
            None,
            Vec::new(),
        )),
    }
}

fn create_select(table: String) -> Select {
    Select {
        distinct: false,
        top: None,
        projection: Vec::new(),
        into: None,
        from: create_from(table),
        lateral_views: Vec::new(),
        selection: None,
        group_by: Vec::new(),
        cluster_by: Vec::new(),
        distribute_by: Vec::new(),
        sort_by: Vec::new(),
        having: None,
    }
}

// This function needs more work
// It needs to define multi tables and joins
// I assume we will need to define expressions for the columns instead of strings
fn create_from(table: String) -> Vec<TableWithJoins> {
    let ident = Ident {
        value: table,
        quote_style: None,
    };

    let table_factor = TableFactor::Table {
        name: ObjectName(vec![ident]),
        alias: None,
        args: Vec::new(),
        with_hints: Vec::new(),
    };

    let table = TableWithJoins {
        relation: table_factor,
        joins: Vec::new(),
    };

    vec![table]
}
