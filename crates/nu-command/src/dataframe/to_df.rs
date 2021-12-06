use super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span,
};

#[derive(Clone)]
pub struct ToDataFrame;

impl Command for ToDataFrame {
    fn name(&self) -> &str {
        "to df"
    }

    fn usage(&self) -> &str {
        "Converts a List, Table or Dictionary into a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes a dictionary and creates a dataframe",
                example: "[[a b];[1 2] [3 4]] | to df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![1.into(), 3.into()]),
                        Column::new("b".to_string(), vec![2.into(), 4.into()]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
            Example {
                description: "Takes a list of tables and creates a dataframe",
                example: "[[1 2 a] [3 4 b] [5 6 c]] | to df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("0".to_string(), vec![1.into(), 3.into(), 5.into()]),
                        Column::new("1".to_string(), vec![2.into(), 4.into(), 6.into()]),
                        Column::new(
                            "2".to_string(),
                            vec![
                                "a".to_string().into(),
                                "b".to_string().into(),
                                "c".to_string().into(),
                            ],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
            Example {
                description: "Takes a list and creates a dataframe",
                example: "[a b c] | to df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![
                            "a".to_string().into(),
                            "b".to_string().into(),
                            "c".to_string().into(),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
            Example {
                description: "Takes a list of booleans and creates a dataframe",
                example: "[$true $true $false] | to df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![true.into(), true.into(), false.into()],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::unknown()),
                ),
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        NuDataFrame::try_from_iter(input.into_iter())
            .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(ToDataFrame {})
    }
}
