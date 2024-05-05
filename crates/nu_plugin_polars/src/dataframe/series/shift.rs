use crate::{
    dataframe::values::{NuExpression, NuLazyFrame},
    values::CustomValueSupport,
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
        vec![
            Example {
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
            },
            Example {
                description: "Shifts the values by a given period, fill absent values with 0",
                example:
                    "[1 2 2 3 3] | polars into-df | polars shift 2 --fill 0 | polars collect",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(2),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head);
        let lazy = NuLazyFrame::try_from_value_coerce(plugin, &value)?;
        command_lazy(plugin, engine, call, lazy).map_err(LabeledError::from)
    }
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
            let expr = NuExpression::try_from_value(plugin, fill)?.into_polars();
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
