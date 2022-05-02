use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{Query, Select, SetExpr, Expr};

#[derive(Clone)]
pub struct WhereDb;

impl Command for WhereDb {
    fn name(&self) -> &str {
        "db where"
    }

    fn usage(&self) -> &str {
        "Includes a where statement for a query"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("where", SyntaxShape::Any, "Where expression on the table")
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "where"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "selects a column from a database with a where clause",
            example:
                "db open db.mysql | db select a | db from table_1 | db where ((db col a) > 1) | db describe",
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
        let value: Value = call.req(engine_state, stack, 0)?;
        let expr = ExprDb::try_from_value(value)?.into_native();

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.query = match db.query {
            None => Some(create_query(expr)),
            Some(query) => Some(modify_query(query, expr)),
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn create_query(expression: Expr) -> Query {
    Query {
        with: None,
        body: SetExpr::Select(Box::new(create_select(expression))),
        order_by: Vec::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    }
}

fn modify_query(mut query: Query, expression: Expr) -> Query {
    query.body = match query.body {
        SetExpr::Select(select) => SetExpr::Select(modify_select(select, expression)),
        _ => SetExpr::Select(Box::new(create_select(expression))),
    };

    query
}

fn modify_select(mut select: Box<Select>, expression: Expr) -> Box<Select> {
    select.as_mut().selection = Some(expression);
    select
}

fn create_select(expression: Expr) -> Select {
    Select {
        distinct: false,
        top: None,
        into: None,
        projection: Vec::new(),
        from: Vec::new(),
        lateral_views: Vec::new(),
        selection: Some(expression),
        group_by: Vec::new(),
        cluster_by: Vec::new(),
        distribute_by: Vec::new(),
        sort_by: Vec::new(),
        having: None,
    }
}
