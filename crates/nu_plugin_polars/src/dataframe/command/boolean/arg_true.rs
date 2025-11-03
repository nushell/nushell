use crate::{
    PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginType},
};

use super::super::super::values::{Column, NuDataFrame};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Value,
};
use polars::prelude::{IntoLazy, arg_where, col};

#[derive(Clone)]
pub struct ArgTrue;

impl PluginCommand for ArgTrue {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars arg-true"
    }

    fn description(&self) -> &str {
        "Returns indexes where values are true."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argtrue", "truth", "boolean-true"]
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
            description: "Returns indexes where values are true",
            example: "[false true false] | polars into-df | polars arg-true",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "arg_true".to_string(),
                        vec![Value::test_int(1)],
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
        let metadata = input.metadata();
        command(plugin, engine, call, input)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;
    let columns = df.as_ref().get_column_names();
    if columns.len() > 1 {
        return Err(ShellError::GenericError {
            error: "Error using as series".into(),
            msg: "dataframe has more than one column".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
    }

    match columns.first() {
        Some(column) => {
            let expression = arg_where(col((*column).clone()).eq(true)).alias("arg_true");
            let res: NuDataFrame = df
                .as_ref()
                .clone()
                .lazy()
                .select(&[expression])
                .collect()
                .map_err(|err| ShellError::GenericError {
                    error: "Error creating index column".into(),
                    msg: err.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })?
                .into();

            res.to_pipeline_data(plugin, engine, call.head)
        }
        _ => Err(ShellError::UnsupportedInput {
            msg: "Expected the dataframe to have a column".to_string(),
            input: "".to_string(),
            msg_span: call.head,
            input_span: call.head,
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&ArgTrue)
    }
}
