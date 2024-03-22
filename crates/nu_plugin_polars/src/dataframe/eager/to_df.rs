use crate::{dataframe::values::NuSchema, values::Column, PolarsDataFramePlugin};

use super::super::values::NuDataFrame;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, CustomValue, LabeledError, PipelineData, PluginExample, PluginSignature, Span,
    SyntaxShape, Type, Value,
};
use polars::{
    prelude::{AnyValue, DataType, Field, NamedFrom},
    series::Series,
};

#[derive(Clone)]
pub struct ToDataFrame;

impl PluginCommand for ToDataFrame {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars into-df")
            .usage("Converts a list, table or record into a dataframe.")
            .named(
                "schema",
                SyntaxShape::Record(vec![]),
                r#"Polars Schema in format [{name: str}]. CSV, JSON, and JSONL files"#,
                Some('s'),
            )
            .input_output_type(Type::Any, Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
            .plugin_examples(examples())
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let maybe_schema = call
            .get_flag("schema")?
            .map(|schema| NuSchema::try_from(&schema))
            .transpose()?;

        let df = NuDataFrame::try_from_iter(input.into_iter(), maybe_schema.clone())?;

        Ok(PipelineData::Value(
            df.insert_cache(engine)?.into_value(call.head),
            None,
        ))
    }
}

// todo - fix examples
fn examples() -> Vec<PluginExample> {
    vec![
        PluginExample {
            description: "Takes a dictionary and creates a dataframe".into(),
            example: "[[a b];[1 2] [3 4]] | polars into-df".into(),
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
                .custom_value()
                .to_base_value(Span::test_data())
                .expect("rendering base value should not faile")
            ),
        },
        PluginExample {
            description: "Takes a list of tables and creates a dataframe".into(),
            example: "[[1 2 a] [3 4 b] [5 6 c]] | polars into-df".into(),
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
                .custom_value()
                .to_base_value(Span::test_data())
                .expect("rendering base value should not faile")
            ),
        },
        PluginExample {
            description: "Takes a list and creates a dataframe".into(),
            example: "[a b c] | polars into-df".into(),
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
                .custom_value()
                .to_base_value(Span::test_data())
                .expect("rendering base value should not faile")
            ),
        },
        PluginExample {
            description: "Takes a list of booleans and creates a dataframe".into(),
            example: "[true true false] | polars into-df".into(),
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
                .custom_value()
                .to_base_value(Span::test_data())
                .expect("rendering base value should not faile")
            ),
        },
        PluginExample {
            description: "Convert to a dataframe and provide a schema".into(),
            example: "{a: 1, b: {a: [1 2 3]}, c: [a b c]}| polars into-df -s {a: u8, b: {a: list<u64>}, c: list<str>}".into(),
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
                .custom_value()
                .to_base_value(Span::test_data())
                .expect("rendering base value should not faile")
            ),
        },
        PluginExample {
            description: "Convert to a dataframe and provide a schema that adds a new column".into(),
            example: r#"[[a b]; [1 "foo"] [2 "bar"]] | polars into-df -s {a: u8, b:str, c:i64} | polars fill-null 3"#.into(),
            result: Some(NuDataFrame::try_from_series(vec![
                    Series::new("a", [1u8, 2]),
                    Series::new("b", ["foo", "bar"]),
                    Series::new("c", [3i64, 3]),
                ], Span::test_data())
                .expect("simple df for test should not fail")
                .custom_value()
                .to_base_value(Span::test_data())
                .expect("rendering base value should not faile")
            ),
        }
    ]
}
