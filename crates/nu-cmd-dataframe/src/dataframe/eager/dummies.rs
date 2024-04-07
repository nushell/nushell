use crate::dataframe::values::NuDataFrame;
use nu_engine::command_prelude::*;

use polars::{prelude::*, series::Series};

#[derive(Clone)]
pub struct Dummies;

impl Command for Dummies {
    fn name(&self) -> &str {
        "dfr dummies"
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
                example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr dummies",
                result: Some(
                    NuDataFrame::try_from_series(
                        vec![
                            Series::new("a_1", &[1_u8, 0]),
                            Series::new("a_3", &[0_u8, 1]),
                            Series::new("b_2", &[1_u8, 0]),
                            Series::new("b_4", &[0_u8, 1]),
                        ],
                        Span::test_data(),
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Create new dataframe with dummy variables from a series",
                example: "[1 2 2 3 3] | dfr into-df | dfr dummies",
                result: Some(
                    NuDataFrame::try_from_series(
                        vec![
                            Series::new("0_1", &[1_u8, 0, 0, 0, 0]),
                            Series::new("0_2", &[0_u8, 1, 1, 0, 0]),
                            Series::new("0_3", &[0_u8, 0, 0, 1, 1]),
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let drop_first: bool = call.has_flag(engine_state, stack, "drop-first")?;
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    df.as_ref()
        .to_dummies(None, drop_first)
        .map_err(|e| ShellError::GenericError {
            error: "Error calculating dummies".into(),
            msg: e.to_string(),
            span: Some(call.head),
            help: Some("The only allowed column types for dummies are String or Int".into()),
            inner: vec![],
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(Dummies {})])
    }
}
