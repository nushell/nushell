use super::super::values::NuDataFrame;
use crate::{values::CustomValueSupport, Cacheable, PolarsPlugin};
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

    fn usage(&self) -> &str {
        "Creates a new dataframe with dummy variables."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .switch("drop-first", "Drop first row", Some('d'))
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create new dataframe with dummy variables from a dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars dummies",
                result: Some(
                    NuDataFrame::try_from_series_columns(
                        vec![
                            Series::new("a_1", &[1_u8, 0]),
                            Series::new("a_3", &[0_u8, 1]),
                            Series::new("b_2", &[1_u8, 0]),
                            Series::new("b_4", &[0_u8, 1]),
                        ],
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Create new dataframe with dummy variables from a series",
                example: "[1 2 2 3 3] | polars into-df | polars dummies",
                result: Some(
                    NuDataFrame::try_from_series_columns(
                        vec![
                            Series::new("0_1", &[1_u8, 0, 0, 0, 0]),
                            Series::new("0_2", &[0_u8, 1, 1, 0, 0]),
                            Series::new("0_3", &[0_u8, 0, 0, 1, 1]),
                        ],
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
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
    let df = NuDataFrame::try_from_pipeline(plugin, input, call.head)?;

    let df = df
        .as_ref()
        .to_dummies(None, drop_first)
        .map_err(|e| ShellError::GenericError {
            error: "Error calculating dummies".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: Some("The only allowed column types for dummies are String or Int".into()),
            inner: vec![],
        })?;

    let df = NuDataFrame::new(false, df);
    Ok(PipelineData::Value(
        df.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![Box::new(Dummies {})])
//     }
// }
