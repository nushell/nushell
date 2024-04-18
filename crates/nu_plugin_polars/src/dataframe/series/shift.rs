use crate::{
    dataframe::values::{NuExpression, NuLazyFrame},
    values::{cant_convert_err, CustomValueSupport, PolarsPluginObject, PolarsPluginType},
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

use polars_plan::prelude::lit;

#[derive(Clone)]
pub struct Shift;

impl PluginCommand for Shift {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars shift"
    }

    fn usage(&self) -> &str {
        "Shifts the values by a given period."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("period", SyntaxShape::Int, "shift period")
            .named(
                "fill",
                SyntaxShape::Any,
                "Expression used to fill the null values (lazy df)",
                Some('f'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe or lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shifts the values by a given period",
            example: "[1 2 2 3 3] | polars into-df | polars shift 2 | polars drop-nulls",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "0".to_string(),
                        vec![Value::test_int(1), Value::test_int(2), Value::test_int(2)],
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
        let value = input.into_value(call.head);

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command_eager(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => command_lazy(plugin, engine, call, lazy),
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyGroupBy,
                ],
            )),
        }
        .map_err(LabeledError::from)
    }
}

fn command_eager(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let period: i64 = call.req(0)?;
    let series = df.as_series(call.head)?.shift(period);

    let df = NuDataFrame::try_from_series_vec(vec![series], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let shift: i64 = call.req(0)?;
    let fill: Option<Value> = call.get_flag("fill")?;

    let lazy = lazy.to_polars();

    let lazy: NuLazyFrame = match fill {
        Some(ref fill) => {
            let expr = NuExpression::try_from_value(plugin, fill)?.to_polars();
            lazy.shift_and_fill(lit(shift), expr).into()
        }
        None => lazy.shift(shift).into(),
    };

    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Shift)
    }
}
