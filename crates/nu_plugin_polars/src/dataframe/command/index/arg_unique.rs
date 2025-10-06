use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginType},
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct ArgUnique;

impl PluginCommand for ArgUnique {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars arg-unique"
    }

    fn description(&self) -> &str {
        "Returns indexes for unique values."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argunique", "distinct", "noduplicate", "unrepeated"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (
                    PolarsPluginType::NuDataFrame.into(),
                    PolarsPluginType::NuDataFrame.into(),
                ),
                (
                    PolarsPluginType::NuLazyFrame.into(),
                    PolarsPluginType::NuLazyFrame.into(),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Returns indexes for unique values",
            example: "[1 2 2 3 3] | polars into-df | polars arg-unique",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "arg_unique".to_string(),
                        vec![Value::test_int(0), Value::test_int(1), Value::test_int(3)],
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
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let mut res = df
        .as_series(call.head)?
        .arg_unique()
        .map_err(|e| ShellError::GenericError {
            error: "Error extracting unique values".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?
        .into_series();
    res.rename("arg_unique".into());

    let df = NuDataFrame::try_from_series_vec(vec![res], call.head)?;
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ArgUnique)
    }
}
