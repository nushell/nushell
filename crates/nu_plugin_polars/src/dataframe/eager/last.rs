use crate::{
    values::{to_pipeline_data, Column, CustomValueSupport},
    PolarsPlugin,
};

use super::super::values::{utils::DEFAULT_ROWS, NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct LastDF;

impl PluginCommand for LastDF {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars last"
    }

    fn usage(&self) -> &str {
        "Creates new dataframe with tail rows or creates a last expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional("rows", SyntaxShape::Int, "Number of rows for tail")
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
                description: "Create new dataframe with last rows",
                example: "[[a b]; [1 2] [3 4]] | polars into-df | polars last 1",
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new("a".to_string(), vec![Value::test_int(3)]),
                            Column::new("b".to_string(), vec![Value::test_int(4)]),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .base_value(Span::test_data())
                    .expect("rendering base value should not fail"),
                ),
            },
            Example {
                description: "Creates a last expression from a column",
                example: "polars col a | polars last",
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
            let expr: NuExpression = expr.to_polars().last().into();

            to_pipeline_data(plugin, engine, call.head, expr).map_err(LabeledError::from)
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
    let rows = rows.unwrap_or(DEFAULT_ROWS);

    let res = df.as_ref().tail(Some(rows));
    let res = NuDataFrame::new(false, res);
    to_pipeline_data(plugin, engine, call.head, res)
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
//         let mut engine_state = build_test_engine_state(vec![Box::new(LastDF {})]);
//         test_dataframe_example(&mut engine_state, &LastDF.examples()[0]);
//     }
//
//     #[test]
//     fn test_examples_expression() {
//         let mut engine_state = build_test_engine_state(vec![
//             Box::new(LastDF {}),
//             Box::new(LazyAggregate {}),
//             Box::new(ToLazyGroupBy {}),
//         ]);
//         test_dataframe_example(&mut engine_state, &LastDF.examples()[1]);
//     }
// }