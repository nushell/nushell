use super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct ToDataFrame;

impl Command for ToDataFrame {
    fn name(&self) -> &str {
        "dfr to-df"
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
                example: "[[a b];[1 2] [3 4]] | dfr to-df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::Int(1), Value::Int(3)]),
                        Column::new("b".to_string(), vec![Value::Int(2), Value::Int(4)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes a list of tables and creates a dataframe",
                example: "[[1 2 a] [3 4 b] [5 6 c]] | dfr to-df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "0".to_string(),
                            vec![Value::Int(1), Value::Int(3), Value::Int(5)],
                        ),
                        Column::new(
                            "1".to_string(),
                            vec![Value::Int(2), Value::Int(4), Value::Int(6)],
                        ),
                        Column::new(
                            "2".to_string(),
                            vec![
                                Value::String("a".into()),
                                Value::String("b".into()),
                                Value::String("c".into()),
                            ],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes a list and creates a dataframe",
                example: "[a b c] | dfr to-df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![
                            Value::String("a".into()),
                            Value::String("b".into()),
                            Value::String("c".into()),
                        ],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes a list of booleans and creates a dataframe",
                example: "[true true false] | dfr to-df",
                result: Some(
                    NuDataFrame::try_from_columns(vec![Column::new(
                        "0".to_string(),
                        vec![Value::Bool(true), Value::Bool(true), Value::Bool(false)],
                    )])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
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
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ToDataFrame {})])
    }
}
