use super::super::SQLiteDatabase;
use crate::database::values::dsl::ExprDb;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LimitDb;

impl Command for LimitDb {
    fn name(&self) -> &str {
        "db limit"
    }

    fn usage(&self) -> &str {
        "Limit result from query"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "limit",
                SyntaxShape::Int,
                "Number of rows to extract for query",
            )
            .category(Category::Custom("database".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["database", "limit"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Limits selection from table",
            example: r#"db open db.mysql 
    | db from table_a 
    | db select a 
    | db limit 10 
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
        let limit: Value = call.req(engine_state, stack, 0)?;
        let expr = ExprDb::try_from_value(&limit)?.into_native();

        let mut db = SQLiteDatabase::try_from_pipeline(input, call.head)?;
        db.query = match db.query {
            Some(mut query) => {
                query.limit = Some(expr);
                Some(query)
            }
            None => {
                return Err(ShellError::GenericError(
                    "Connection without query".into(),
                    "The connection needs a query defined".into(),
                    Some(call.head),
                    None,
                    Vec::new(),
                ))
            }
        };

        Ok(db.into_value(call.head).into_pipeline_data())
    }
}
