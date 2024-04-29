use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use crate::{
    dataframe::values::utils::convert_columns_string,
    values::{CustomValueSupport, NuDataFrame},
    PolarsPlugin,
};

use super::super::values::{Column, NuLazyFrame};
use polars::prelude::{col, Expr};

#[derive(Clone)]
pub struct GetDF;

impl PluginCommand for GetDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars get"
    }

    fn usage(&self) -> &str {
        "Creates dataframe with the selected columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("rest", SyntaxShape::Any, "column names to sort dataframe")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns the selected column",
            example: "[[a b]; [1 2] [3 4]] | polars into-lazy | polars get a | polars collect",
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let columns: Vec<Value> = call.rest(0)?;
    let (col_string, _col_span) = convert_columns_string(columns, call.head)?;
    let col_expr: Vec<Expr> = col_string.iter().map(|s| col(s)).collect();

    let df = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let df = df.to_polars().select(col_expr);
    let df = NuLazyFrame::new(df);
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&GetDF)
    }
}
