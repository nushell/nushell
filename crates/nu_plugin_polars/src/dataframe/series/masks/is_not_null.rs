use crate::{
    values::{
        cant_convert_err, to_pipeline_data, CustomValueSupport, PolarsPluginObject,
        PolarsPluginType,
    },
    PolarsPlugin,
};

use super::super::super::values::{Column, NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::IntoSeries;

#[derive(Clone)]
pub struct IsNotNull;

impl PluginCommand for IsNotNull {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars is-not-null"
    }

    fn usage(&self) -> &str {
        "Creates mask where value is not null."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
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
                description: "Create mask where values are not null",
                example: r#"let s = ([5 6 0 8] | polars into-df);
    let res = ($s / $s);
    $res | polars is-not-null"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![Column::new(
                            "is_not_null".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                                Value::test_bool(true),
                            ],
                        )],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Creates a is not null expression from a column",
                example: "polars col a | polars is-not-null",
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

        match PolarsPluginObject::try_from_value(plugin, &value)? {
            PolarsPluginObject::NuDataFrame(df) => command(plugin, engine, call, df),
            PolarsPluginObject::NuLazyFrame(lazy) => {
                command(plugin, engine, call, lazy.collect(call.head)?)
            }
            PolarsPluginObject::NuExpression(expr) => {
                let expr: NuExpression = expr.to_polars().is_not_null().into();
                to_pipeline_data(plugin, engine, call.head, expr)
            }
            _ => Err(cant_convert_err(
                &value,
                &[
                    PolarsPluginType::NuDataFrame,
                    PolarsPluginType::NuLazyFrame,
                    PolarsPluginType::NuExpression,
                ],
            )),
        }
        .map_err(LabeledError::from)
    }
}

fn command(
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    df: NuDataFrame,
) -> Result<PipelineData, ShellError> {
    let mut res = df.as_series(call.head)?.is_not_null();
    res.rename("is_not_null");

    let df = NuDataFrame::try_from_series_vec(vec![res.into_series()], call.head)?;
    to_pipeline_data(plugin, engine, call.head, df)
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::dataframe::lazy::aggregate::LazyAggregate;
//     use crate::dataframe::lazy::groupby::ToLazyGroupBy;
//     use crate::dataframe::test_dataframe::{build_test_engine_state, test_dataframe_example};
//
//     #[test]
//     fn test_examples_dataframe() {
//         let mut engine_state = build_test_engine_state(vec![Box::new(IsNotNull {})]);
//         test_dataframe_example(&mut engine_state, &IsNotNull.examples()[0]);
//     }
//
//     #[test]
//     fn test_examples_expression() {
//         let mut engine_state = build_test_engine_state(vec![
//             Box::new(IsNotNull {}),
//             Box::new(LazyAggregate {}),
//             Box::new(ToLazyGroupBy {}),
//         ]);
//         test_dataframe_example(&mut engine_state, &IsNotNull.examples()[1]);
//     }
// }
