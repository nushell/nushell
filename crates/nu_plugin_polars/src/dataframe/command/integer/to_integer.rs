use crate::{
    PolarsPlugin,
    values::{
        Column, CustomValueSupport, NuDataFrame, NuDataType, NuExpression, PolarsPluginObject,
        PolarsPluginType, cant_convert_err,
    },
};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use polars::prelude::{DataType, Expr, lit};

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
            .switch("strict", "Raises an error as opposed to converting to null", Some('s'))
            .optional(
                "base",
                SyntaxShape::Any,
                "An integer or expression representing the base (radix) of the number system (default is 10)",
            )
            .optional(
                "dtype",
                SyntaxShape::Any,
                "Data type to cast to (defaults is i64)",
                )
            .input_output_type(
                PolarsPluginType::NuExpression.into(),
                PolarsPluginType::NuExpression.into(),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
    let strict: bool = call.has_flag("strict")?;

    let base: Expr = call
        .opt(0)?
        .map(|ref v| NuExpression::try_from_value(plugin, v))
        .transpose()?
        .map(|e| e.into_polars())
        .unwrap_or(lit(10));

    let dtype: Option<DataType> = call
        .opt(1)?
        .map(|ref v| NuDataType::try_from_value(plugin, v))
        .transpose()?
        .map(|dt| dt.to_polars());

    let res: NuExpression = expr
        .into_polars()
        .str()
        .to_integer(base, dtype, strict)
        .into();

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
