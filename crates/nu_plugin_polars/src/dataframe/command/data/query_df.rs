use super::sql_context::SQLContext;
use crate::PolarsPlugin;
use crate::dataframe::values::Column;
use crate::dataframe::values::NuLazyFrame;
use crate::values::CustomValueSupport;
use crate::values::NuDataFrame;
use crate::values::PolarsPluginType;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

// attribution:
// sql_context.rs, and sql_expr.rs were copied from polars-sql. thank you.
// maybe we should just use the crate at some point but it's not published yet.
// https://github.com/pola-rs/polars/tree/master/polars-sql

#[derive(Clone)]
pub struct QueryDf;

impl PluginCommand for QueryDf {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars query"
    }

    fn description(&self) -> &str {
        "Query dataframe using SQL. Note: The dataframe is always named 'df' in your query's from clause."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("sql", SyntaxShape::String, "sql query")
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["dataframe", "sql", "search"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Query dataframe using SQL",
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars query 'select a from df'",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "a".to_string(),
                        vec![Value::test_int(1), Value::test_int(3)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let sql_query: String = call.req(0)?;
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let mut ctx = SQLContext::new();
    ctx.register("df", &df.df);
    let df_sql = ctx
        .execute(&sql_query)
        .map_err(|e| ShellError::GenericError {
            error: "Dataframe Error".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;
    let lazy = NuLazyFrame::new(!df.from_lazy, df_sql);
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&QueryDf)
    }
}
