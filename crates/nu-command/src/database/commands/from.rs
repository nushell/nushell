use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape,
};
use sqlparser::ast::{Ident, ObjectName, Query, Select, SetExpr, TableFactor, TableWithJoins};

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
        db.query = match db.query {
            None => Some(create_query(table)),
            Some(query) => Some(modify_query(query, table)),
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn create_query(table: String) -> Query {
    Query {
        with: None,
        body: SetExpr::Select(Box::new(create_select(table))),
        order_by: Vec::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    }
}

fn modify_query(mut query: Query, table: String) -> Query {
    query.body = match query.body {
        SetExpr::Select(select) => SetExpr::Select(Box::new(modify_select(select, table))),
        _ => SetExpr::Select(Box::new(create_select(table))),
    };

    query
}

fn modify_select(select: Box<Select>, table: String) -> Select {
    Select {
        from: create_from(table),
        ..select.as_ref().clone()
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
