use crate::{
    dataframe::values::NuSchema,
    values::{Column, CustomValueSupport},
    PolarsPlugin,
};

use crate::values::NuDataFrame;

use log::debug;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
use polars::{
    prelude::{AnyValue, DataType, Field, NamedFrom},
    series::Series,
};

#[derive(Clone)]
pub struct ToDataFrame;

impl PluginCommand for ToDataFrame {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars into-df"
    }

    fn description(&self) -> &str {
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
            .switch(
                "as-columns",
                r#"When input shape is record of lists, treat each list as column values."#,
                Some('c'),
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Takes a dictionary and creates a dataframe",
                example: "[[a b];[1 2] [3 4]] | polars into-df",
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
                description: "Takes a record of lists and creates a dataframe",
                example: "{a: [1 3], b: [2 4]} | polars into-df --as-columns",
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
                example: "[[1 2 a] [3 4 b] [5 6 c]] | polars into-df",
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
                example: "[a b c] | polars into-df",
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
                example: "[true true false] | polars into-df",
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
                example: "[[a b c]; [1 {d: [1 2 3]} [10 11 12] ]]| polars into-df -s {a: u8, b: {d: list<u64>}, c: list<u8>}",
                result: Some(
                    NuDataFrame::try_from_series_vec(vec![
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
                            let vals = vec![AnyValue::List(Series::new("c", &[10, 11, 12]))];
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
                example: r#"[[a b]; [1 "foo"] [2 "bar"]] | polars into-df -s {a: u8, b:str, c:i64} | polars fill-null 3"#,
                result: Some(NuDataFrame::try_from_series_vec(vec![
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
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let maybe_schema = call
            .get_flag("schema")?
            .map(|schema| NuSchema::try_from(&schema))
            .transpose()?;

        debug!("schema: {:?}", maybe_schema);

        let maybe_as_columns = call.has_flag("as-columns")?;

        let df = if !maybe_as_columns {
            NuDataFrame::try_from_iter(plugin, input.into_iter(), maybe_schema.clone())?
        } else {
            match &input {
                PipelineData::Value(Value::Record { val, .. }, _) => {
                    let items: Result<Vec<(String, Vec<Value>)>, &str> = val
                        .iter()
                        .map(|(k, v)| match v.to_owned().into_list() {
                            Ok(v) => Ok((k.to_owned(), v)),
                            _ => Err("error"),
                        })
                        .collect();
                    match items {
                        Ok(items) => {
                            let columns = items
                                .iter()
                                .map(|(k, v)| Column::new(k.to_owned(), v.to_owned()))
                                .collect::<Vec<Column>>();
                            NuDataFrame::try_from_columns(columns, maybe_schema)?
                        }
                        Err(e) => {
                            debug!(
                                "Failed to build with multiple columns, attempting as series. failure:{e}"
                            );
                            NuDataFrame::try_from_iter(
                                plugin,
                                input.into_iter(),
                                maybe_schema.clone(),
                            )?
                        }
                    }
                }
                _ => {
                    debug!("Other input: {input:?}");
                    NuDataFrame::try_from_iter(plugin, input.into_iter(), maybe_schema.clone())?
                }
            }
        };

        df.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
    }
}

#[cfg(test)]
mod test {
    use crate::test::test_polars_plugin_command;

    use super::*;
    use nu_protocol::ShellError;

    #[test]
    fn test_into_df() -> Result<(), ShellError> {
        test_polars_plugin_command(&ToDataFrame)
    }
}
