use crate::{values::Column, Cacheable, CustomValueSupport, PolarsPlugin};

use super::super::values::{NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct FirstDF;

impl PluginCommand for FirstDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars first"
    }

    fn usage(&self) -> &str {
        "Show only the first number of rows or create a first expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "rows",
                SyntaxShape::Int,
                "starting from the front, the number of rows to return",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Return the first row of a dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars first",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(1)]),
                            Column::new("b".to_string(), vec![Value::test_int(2)]),
                        ],
                        None,
                    )
                    .expect("should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Return the first two rows of a dataframe",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars first 2",
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
                    .expect("should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Creates a first expression from a column",
                example: "polars col a | polars first",
                result: None,
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
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(plugin, &value)?;
            command(plugin, engine, call, df).map_err(|e| e.into())
        } else {
            let expr = NuExpression::try_from_value(plugin, &value)?;
            let expr: NuExpression = expr.to_polars().first().into();

            Ok(PipelineData::Value(
                expr.cache(plugin, engine)?.into_value(call.head),
                None,
            ))
        }
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.opt(0)?;
    let rows = rows.unwrap_or(1);

    let res = df.as_ref().head(Some(rows));
    let res = NuDataFrame::new(false, res);

    Ok(PipelineData::Value(
        res.cache(plugin, engine)?.into_value(call.head),
        None,
    ))
}

// todo - fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
//     use super::*;
//     use crate::dataframe::lazy::aggregate::LazyAggregate;
//     use crate::dataframe::lazy::groupby::ToLazyGroupBy;
//
//     #[test]
//     fn test_examples_dataframe() {
//         let mut engine_state = build_test_engine_state(vec![Box::new(FirstDF {})]);
//         test_dataframe_example(&mut engine_state, &FirstDF.examples()[0]);
//         test_dataframe_example(&mut engine_state, &FirstDF.examples()[1]);
//     }
//
//     #[test]
//     fn test_examples_expression() {
//         let mut engine_state = build_test_engine_state(vec![
//             Box::new(FirstDF {}),
//             Box::new(LazyAggregate {}),
//             Box::new(ToLazyGroupBy {}),
//         ]);
//         test_dataframe_example(&mut engine_state, &FirstDF.examples()[2]);
//     }
// }
