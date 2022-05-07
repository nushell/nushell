use crate::database::values::dsl::ExprDb;

use super::super::SQLiteDatabase;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use sqlparser::ast::{SetExpr, Statement};

#[derive(Clone)]
pub struct GroupByDb;

impl Command for GroupByDb {
    fn name(&self) -> &str {
        "db group-by"
    }

    fn usage(&self) -> &str {
        "Group by query"
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
        vec![Example {
            description: "orders query by a column",
            example: r#"db open db.mysql 
    | db from table_a 
    | db select a 
    | db group-by a 
    | db describe"#,
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
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = ExprDb::extract_exprs(value)?;

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        match db.statement.as_mut() {
            Some(statement) => match statement {
                Statement::Query(ref mut query) => match &mut query.body {
                    SetExpr::Select(ref mut select) => select.group_by = expressions,
                    s => {
                        return Err(ShellError::GenericError(
                            "Connection doesnt define a select".into(),
                            format!("Expected a connection with select query. Got {}", s),
                            Some(call.head),
                            None,
                            Vec::new(),
                        ))
                    }
                },
                s => {
                    return Err(ShellError::GenericError(
                        "Connection doesnt define a query".into(),
                        format!("Expected a connection with query. Got {}", s),
                        Some(call.head),
                        None,
                        Vec::new(),
                    ))
                }
            },
            None => {
                return Err(ShellError::GenericError(
                    "Connection without statement".into(),
                    "The connection needs a statement defined".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}
