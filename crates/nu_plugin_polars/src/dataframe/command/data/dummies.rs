use crate::values::NuDataFrame;
use crate::{PolarsPlugin, values::CustomValueSupport};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type,
};
use polars::{prelude::*, series::Series};

#[derive(Clone)]
pub struct Dummies;

impl PluginCommand for Dummies {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars dummies"
    }

    fn description(&self) -> &str {
        "Creates a new dataframe with dummy variables."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("drop-first", "Drop first row", Some('d'))
            .switch("drop-nulls", "Drop nulls", Some('n'))
            .switch("separator", "Optional separator", Some('s'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create new dataframe with dummy variables from a dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars dummies",
                result: Some(
                    NuDataFrame::try_from_series_vec(
                        vec![
                            Series::new("a_1".into(), &[1_u8, 0]),
                            Series::new("a_3".into(), &[0_u8, 1]),
                            Series::new("b_2".into(), &[1_u8, 0]),
                            Series::new("b_4".into(), &[0_u8, 1]),
                        ],
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create new dataframe with dummy variables from a series",
                example: "[1 2 2 3 3] | polars into-df | polars dummies",
                result: Some(
                    NuDataFrame::try_from_series_vec(
                        vec![
                            Series::new("0_1".into(), &[1_u8, 0, 0, 0, 0]),
                            Series::new("0_2".into(), &[0_u8, 1, 1, 0, 0]),
                            Series::new("0_3".into(), &[0_u8, 0, 0, 1, 1]),
                        ],
                        Span::test_data(),
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
    let drop_first: bool = call.has_flag("drop-first")?;
    let drop_nulls: bool = call.has_flag("drop-nulls")?;
    let separator: Option<String> = call.get_flag("separator")?;
    let df = NuDataFrame::try_from_pipeline_coerce(plugin, input, call.head)?;

    let polars_df = df
        .as_ref()
        .to_dummies(separator.as_deref(), drop_first, drop_nulls)
        .map_err(|e| ShellError::GenericError {
            error: "Error calculating dummies".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: Some("The only allowed column types for dummies are String or Int".into()),
            inner: vec![],
        })?;

    let df: NuDataFrame = polars_df.into();
    df.to_pipeline_data(plugin, engine, call.head)
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;

    #[test]
    fn test_examples() -> Result<(), ShellError> {
        test_polars_plugin_command(&Dummies)
    }
}
