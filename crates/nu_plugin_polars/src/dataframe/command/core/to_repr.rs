use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};

use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, NuLazyFrame, PolarsPluginType, cant_convert_err},
};

use crate::values::NuDataFrame;

#[derive(Clone)]
pub struct ToRepr;

impl PluginCommand for ToRepr {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-repr"
    }

    fn description(&self) -> &str {
        "Display a dataframe in its repr format."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![(Type::Custom("dataframe".into()), Type::String)])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Shows dataframe in repr format",
                example: "[[a b]; [2025-01-01 2] [2025-01-02 4]] | polars into-df | polars into-repr",
                result: Some(Value::string(
                    r#"
shape: (2, 2)
┌─────────────────────────┬─────┐
│ a                       ┆ b   │
│ ---                     ┆ --- │
│ datetime[ns, UTC]       ┆ i64 │
╞═════════════════════════╪═════╡
│ 2025-01-01 00:00:00 UTC ┆ 2   │
│ 2025-01-02 00:00:00 UTC ┆ 4   │
└─────────────────────────┴─────┘"#
                        .trim(),
                    Span::test_data(),
                )),
            },
            Example {
                description: "Shows lazy dataframe in repr format",
                example: "[[a b]; [2025-01-01 2] [2025-01-02 4]] | polars into-lazy | polars into-repr",
                result: Some(Value::string(
                    r#"
shape: (2, 2)
┌─────────────────────────┬─────┐
│ a                       ┆ b   │
│ ---                     ┆ --- │
│ datetime[ns, UTC]       ┆ i64 │
╞═════════════════════════╪═════╡
│ 2025-01-01 00:00:00 UTC ┆ 2   │
│ 2025-01-02 00:00:00 UTC ┆ 4   │
└─────────────────────────┴─────┘"#
                        .trim(),
                    Span::test_data(),
                )),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head)?;
        if NuDataFrame::can_downcast(&value) || NuLazyFrame::can_downcast(&value) {
            dataframe_command(plugin, call, value)
        } else {
            Err(cant_convert_err(
                &value,
                &[PolarsPluginType::NuDataFrame, PolarsPluginType::NuLazyFrame],
            ))
        }
        .map_err(|e| e.into())
    }
}

fn dataframe_command(
    plugin: &PolarsPlugin,
    call: &EvaluatedCall,
    input: Value,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_value_coerce(plugin, &input, call.head)?;
    let value = Value::string(format!("{df}"), call.head);
    Ok(PipelineData::value(value, None))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToRepr)
    }
}
