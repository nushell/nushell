use crate::{
    dataframe::{utils::extract_strings, values::NuLazyFrame},
    values::{cant_convert_err, CustomValueSupport, PolarsPluginObject, PolarsPluginType},
    PolarsPlugin,
};

use super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{IntoSeries, UniqueKeepStrategy};

#[derive(Clone)]
pub struct Unique;

impl PluginCommand for Unique {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars unique"
    }

    fn usage(&self) -> &str {
        "Returns unique values from a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "subset",
                SyntaxShape::Any,
                "Subset of column(s) to use to maintain rows (lazy df)",
                Some('s'),
            )
            .switch(
                "last",
                "Keeps last unique value. Default keeps first value (lazy df)",
                Some('l'),
            )
            .switch(
                "maintain-order",
                "Keep the same order as the original DataFrame (lazy df)",
                Some('k'),
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
                description: "Returns unique values from a series",
                example: "[2 2 2 2 2] | polars into-df | polars unique",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new("0".to_string(), vec![Value::test_int(2)])],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Creates a is unique expression from a column",
                example: "col a | unique",
                result: None,
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
    let series = df.as_series(call.head)?;

    let res = series.unique().map_err(|e| ShellError::GenericError {
        error: "Error calculating unique values".into(),
        msg: e.to_string(),
        span: Some(call.head),
        help: Some("The str-slice command can only be used with string columns".into()),
        inner: vec![],
    })?;

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

fn command_lazy(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    lazy: NuLazyFrame,
) -> Result<PipelineData, ShellError> {
    let last = call.has_flag("last")?;
    let maintain = call.has_flag("maintain-order")?;

    let subset: Option<Value> = call.get_flag("subset")?;
    let subset = match subset {
        Some(value) => Some(extract_strings(value)?),
        None => None,
    };

    let strategy = if last {
        UniqueKeepStrategy::Last
    } else {
        UniqueKeepStrategy::First
    };

    let lazy = lazy.to_polars();
    let lazy: NuLazyFrame = if maintain {
        lazy.unique(subset, strategy).into()
    } else {
        lazy.unique_stable(subset, strategy).into()
    };
    lazy.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Unique)
    }
}
