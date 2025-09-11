use crate::{
    PolarsPlugin,
    values::{
        Column, CustomValueSupport, NuDataFrame, NuExpression, PolarsPluginObject,
        PolarsPluginType, cant_convert_err,
    },
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::DataType;

#[derive(Clone)]
pub struct ToDecimal;

impl PluginCommand for ToDecimal {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars decimal"
    }

    fn description(&self) -> &str {
        "Converts a string column into a decimal column"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["expression", "decimal", "float"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "infer_length",
                SyntaxShape::Int,
                "Number of decimal points to infer",
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Modifies strings to decimal",
            example: "[[a b]; [1, '2.4']] | polars into-df | polars select (polars col b | polars decimal 2) | polars collect",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new("b".to_string(), vec![Value::test_float(2.40)])],
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
    let infer_length: usize = call.req(0)?;
    let res: NuExpression = expr
        .into_polars()
        .str()
        .to_decimal(infer_length)
        // since there isn't a good way to support actual large decimal types
        // in nushell, just cast it to an f64.
        .cast(DataType::Float64)
        .into();
    res.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToDecimal)
    }
}
