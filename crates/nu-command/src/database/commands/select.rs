use super::{super::SQLiteDatabase, super::values::dsl::SelectDb};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{Query, Select, SelectItem, SetExpr};

#[derive(Clone)]
pub struct ProjectionDb;

impl Command for ProjectionDb {
    fn name(&self) -> &str {
        "db select"
    }

    fn usage(&self) -> &str {
        "Creates a select statement for a DB"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "select",
                SyntaxShape::Any,
                "Select expression(s) on the table",
            )
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "select"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "selects a column from a database",
                example: "db open db.mysql | db select a",
                result: None,
            },
            Example {
                description: "selects columns from a database",
                example: "db open db.mysql | db select a b c",
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
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List { vals, span: call.head };
        let projection = SelectDb::extract_selects(value)?;

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.query = match db.query {
            None => Some(create_query(projection)),
            Some(query) => Some(modify_query(query, projection)),
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn create_query(expressions: Vec<SelectItem>) -> Query {
    Query {
        with: None,
        body: SetExpr::Select(Box::new(create_select(expressions))),
        order_by: Vec::new(),
        limit: None,
        offset: None,
        fetch: None,
        lock: None,
    }
}

fn modify_query(mut query: Query, expressions: Vec<SelectItem>) -> Query {
    query.body = match query.body {
        SetExpr::Select(select) => SetExpr::Select(Box::new(modify_select(select, expressions))),
        _ => SetExpr::Select(Box::new(create_select(expressions))),
    };

    query
}

fn modify_select(select: Box<Select>, projection: Vec<SelectItem>) -> Select {
    Select {
        projection,
        ..select.as_ref().clone()
    }
}

fn create_select(projection: Vec<SelectItem>) -> Select {
    Select {
        distinct: false,
        top: None,
        projection,
        into: None,
        from: Vec::new(),
        lateral_views: Vec::new(),
        selection: None,
        group_by: Vec::new(),
        cluster_by: Vec::new(),
        distribute_by: Vec::new(),
        sort_by: Vec::new(),
        having: None,
    }
}

