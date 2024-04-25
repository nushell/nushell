use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use crate::values::{CustomValueSupport, NuLazyFrame};
use crate::PolarsPlugin;

use super::super::values::utils::convert_columns;
use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct DropDF;

impl PluginCommand for DropDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars drop"
    }

    fn usage(&self) -> &str {
        "Creates a new dataframe by dropping the selected columns."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("rest", SyntaxShape::Any, "column names to be dropped")
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "[[a b]; [1 2] [3 4]] | polars into-df | polars drop a | polars collect",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(4)],
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
    let (col_string, _col_span) = convert_columns(columns, call.head)?;

    let df = NuLazyFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let polars_df = df.to_polars().drop(col_string.iter().map(|s| &s.item));
    let final_df = NuLazyFrame::new(false, polars_df);

    final_df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&DropDF)
    }
}
