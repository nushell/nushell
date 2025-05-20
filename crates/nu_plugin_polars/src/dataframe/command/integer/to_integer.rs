use crate::{
    PolarsPlugin,
    values::{
        Column, CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginObject,
        PolarsPluginType, cant_convert_err,
    },
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::lit;

#[derive(Clone)]
pub struct ToInteger;

impl PluginCommand for ToInteger {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars integer"
    }

    fn description(&self) -> &str {
        "Converts a string column into a integer column"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["expression", "integer", "float"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Modifies strings to integer",
            example: "[[a b]; [1, '2']] | polars into-df | polars select (polars col b | polars integer) | polars collect",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new("b".to_string(), vec![Value::test_int(2)])],
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
        let value = input.into_value(call.head)?;
        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuExpression(expr) => command(plugin, engine, call, expr),
            _ => Err(cant_convert_err(&value, &[PolarsPluginType::NuExpression])),
        }
        .map_err(LabeledError::from)
        .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    expr: NuExpression,
) -> Result<PipelineData, ShellError> {
    let res: NuExpression = expr.into_polars().str().to_integer(lit(10), false).into();
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToInteger)
    }
}
