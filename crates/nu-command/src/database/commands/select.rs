use super::{super::values::dsl::SelectDb, super::SQLiteDatabase};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};
use sqlparser::ast::{Query, Select, SelectItem, SetExpr, Statement};

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
                example: "db open db.mysql | db select a | db describe",
                result: None,
            },
            Example {
                description: "selects columns from a database",
                example: "db open db.mysql | db select a b c | db describe",
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
        let value = Value::List {
            vals,
            span: call.head,
        };
        let projection = SelectDb::extract_selects(value)?;

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.statement = match db.statement {
            None => Some(create_statement(projection)),
            Some(statement) => Some(modify_statement(statement, projection, call.head)?),
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}

fn create_statement(expressions: Vec<SelectItem>) -> Statement {
    let query = Query {
        with: None,
        body: SetExpr::Select(Box::new(create_select(expressions))),
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
    expressions: Vec<SelectItem>,
    span: Span,
) -> Result<Statement, ShellError> {
    match statement {
        Statement::Query(ref mut query) => {
            match query.body {
                SetExpr::Select(ref mut select) => select.as_mut().projection = expressions,
                _ => {
                    query.as_mut().body = SetExpr::Select(Box::new(create_select(expressions)));
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
