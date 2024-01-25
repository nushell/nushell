use crate::dataframe::values::NuSchema;

use super::super::values::{Column, NuDataFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Record, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ToDataFrame;

impl Command for ToDataFrame {
    fn name(&self) -> &str {
        "dfr into-df"
    }

    fn usage(&self) -> &str {
        "Converts a list, table or record into a dataframe."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .named(
                "schema",
                SyntaxShape::Record(vec![]),
                r#"Polars Schema in format [{name: str}]. CSV, JSON, and JSONL files"#,
                Some('s'),
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes a dictionary and creates a dataframe",
                example: "[[a b];[1 2] [3 4]] | dfr into-df",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(1), Value::test_int(3)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes a list of tables and creates a dataframe",
                example: "[[1 2 a] [3 4 b] [5 6 c]] | dfr into-df",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "0".to_string(),
                                vec![Value::test_int(1), Value::test_int(3), Value::test_int(5)],
                            ),
                            Column::new(
                                "1".to_string(),
                                vec![Value::test_int(2), Value::test_int(4), Value::test_int(6)],
                            ),
                            Column::new(
                                "2".to_string(),
                                vec![
                                    Value::test_string("a"),
                                    Value::test_string("b"),
                                    Value::test_string("c"),
                                ],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes a list and creates a dataframe",
                example: "[a b c] | dfr into-df",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_string("a"),
                                Value::test_string("b"),
                                Value::test_string("c"),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Takes a list of booleans and creates a dataframe",
                example: "[true true false] | dfr into-df",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "0".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Convert to a dataframe and provide a schema",
                example: "{a: 1, b: {a: [1 2 3]}, c: [a b c]}| dfr into-df -s {a: u8, b: {a: list[u64]}, c: list[str]} | dfr schema",
                result: Some(Value::record(
                    Record::from_raw_cols_vals(
                        vec!["a".to_string(), "b".to_string(), "c".to_string()],
                        vec![
                            Value::string("u8", Span::test_data()),
                            Value::string("struct[1]", Span::test_data()),
                            Value::string("list[str]", Span::test_data()),
                        ],
                    ),
                    Span::test_data(),
                )),
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
        let maybe_schema = call
            .get_flag(engine_state, stack, "schema")?
            .map(|schema| NuSchema::try_from(&schema))
            .transpose()?;

        NuDataFrame::try_from_iter(input.into_iter(), maybe_schema)
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
