use super::super::values::NuDataFrame;
use crate::dataframe::values::Column;
use crate::dataframe::{eager::SQLContext, values::NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

// attribution:
// sql_context.rs, and sql_expr.rs were copied from polars-sql. thank you.
// maybe we should just use the crate at some point but it's not published yet.
// https://github.com/pola-rs/polars/tree/master/polars-sql

#[derive(Clone)]
pub struct QueryDf;

impl Command for QueryDf {
    fn name(&self) -> &str {
        "query df"
    }

    fn usage(&self) -> &str {
        "Query dataframe using SQL. Note: The dataframe is always named 'df' in your query's from clause."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("sql", SyntaxShape::String, "sql query")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dataframe", "sql", "search"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Query dataframe using SQL",
            example: "[[a b]; [1 2] [3 4]] | into df | query df 'select a from df'",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "a".to_string(),
                    vec![Value::test_int(1), Value::test_int(3)],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let sql_query: String = call.req(engine_state, stack, 0)?;
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let mut ctx = SQLContext::new();
    ctx.register("df", &df.df);
    let df_sql = ctx.execute(&sql_query).map_err(|e| {
        ShellError::GenericError(
            "Dataframe Error".into(),
            e.to_string(),
            Some(call.head),
            None,
            Vec::new(),
        )
    })?;
    let lazy = NuLazyFrame::new(false, df_sql);

    let eager = lazy.collect(call.head)?;
    let value = Value::CustomValue {
        val: Box::new(eager),
        span: call.head,
    };

    Ok(PipelineData::Value(value, None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(QueryDf {})])
    }
}
