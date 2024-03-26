use crate::dataframe::values::{Column, NuDataFrame, NuSchema};
use nu_engine::command_prelude::*;

use polars::prelude::*;

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
                example: "{a: 1, b: {a: [1 2 3]}, c: [a b c]}| dfr into-df -s {a: u8, b: {a: list<u64>}, c: list<str>}",
                result: Some(
                    NuDataFrame::try_from_series(vec![
                        Series::new("a", &[1u8]),
                        {
                            let dtype = DataType::Struct(vec![Field::new("a", DataType::List(Box::new(DataType::UInt64)))]);
                            let vals = vec![AnyValue::StructOwned(
                                Box::new((vec![AnyValue::List(Series::new("a", &[1u64, 2, 3]))], vec![Field::new("a", DataType::String)]))); 1];
                            Series::from_any_values_and_dtype("b", &vals, &dtype, false)
                                .expect("Struct series should not fail")
                        },
                        {
                            let dtype = DataType::List(Box::new(DataType::String));
                            let vals = vec![AnyValue::List(Series::new("c", &["a", "b", "c"]))];
                            Series::from_any_values_and_dtype("c", &vals, &dtype, false)
                                .expect("List series should not fail")
                        }
                    ], Span::test_data())
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Convert to a dataframe and provide a schema that adds a new column",
                example: r#"[[a b]; [1 "foo"] [2 "bar"]] | dfr into-df -s {a: u8, b:str, c:i64} | dfr fill-null 3"#,
                result: Some(NuDataFrame::try_from_series(vec![
                        Series::new("a", [1u8, 2]),
                        Series::new("b", ["foo", "bar"]),
                        Series::new("c", [3i64, 3]),
                    ], Span::test_data())
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            }
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

        let df = NuDataFrame::try_from_iter(input.into_iter(), maybe_schema.clone())?;

        Ok(PipelineData::Value(
            NuDataFrame::into_value(df, call.head),
            None,
        ))
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
