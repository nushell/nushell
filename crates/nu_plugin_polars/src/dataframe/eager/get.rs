use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use crate::{
    dataframe::values::utils::convert_columns_string,
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

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
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars get a",
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
    let (col_string, col_span) = convert_columns_string(columns, call.head)?;

    let df = NuDataFrame::try_from_pipeline(plugin, input, call.head)?;

    let df = df
        .as_ref()
        .select(col_string)
        .map_err(|e| ShellError::GenericError {
            error: "Error selecting columns".into(),
            msg: e.to_string(),
            span: Some(col_span),
            help: None,
            inner: vec![],
        })?;
    let df = NuDataFrame::new(false, df);
    to_pipeline_data(plugin, engine, call.head, df)
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
