use crate::{values::CustomValueSupport, PolarsPlugin};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{IntoSeries, SortOptions};

#[derive(Clone)]
pub struct ArgSort;

impl PluginCommand for ArgSort {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars arg-sort"
    }

    fn description(&self) -> &str {
        "Returns indexes for a sorted series."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argsort", "order", "arrange"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("reverse", "reverse order", Some('r'))
            .switch("nulls-last", "nulls ordered last", Some('n'))
            .switch(
                "maintain-order",
                "maintain order on sorted items",
                Some('m'),
            )
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Returns indexes for a sorted series",
                example: "[1 2 2 3 3] | polars into-df | polars arg-sort",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "arg_sort".to_string(),
                            vec![
                                Value::test_int(0),
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(3),
                                Value::test_int(4),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Returns indexes for a sorted series",
                example: "[1 2 2 3 3] | polars into-df | polars arg-sort --reverse",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "arg_sort".to_string(),
                            vec![
                                Value::test_int(3),
                                Value::test_int(4),
                                Value::test_int(1),
                                Value::test_int(2),
                                Value::test_int(0),
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
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let sort_options = SortOptions {
        descending: call.has_flag("reverse")?,
        nulls_last: call.has_flag("nulls-last")?,
        multithreaded: true,
        maintain_order: call.has_flag("maintain-order")?,
        // todo - expose limit
        limit: None,
    };

    let mut res = df
        .as_series(call.head)?
        .arg_sort(sort_options)
        .into_series();
    res.rename("arg_sort".into());

    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ArgSort)
    }
}
