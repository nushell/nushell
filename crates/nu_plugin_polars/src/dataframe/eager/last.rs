use crate::PolarsDataFramePlugin;

use super::super::values::{utils::DEFAULT_ROWS, NuDataFrame, NuExpression};
use nu_plugin::{EngineInterface, EvaluatedCall, LabeledError, PluginCommand};
use nu_protocol::{
    Category, PipelineData, PluginExample, PluginSignature, ShellError, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct LastDF;

impl PluginCommand for LastDF {
    type Plugin = PolarsDataFramePlugin;

    fn signature(&self) -> PluginSignature {
        PluginSignature::build("polars last")
            .usage("Creates new dataframe with tail rows or creates a last expression.")
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
            .plugin_examples(examples())
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value = input.into_value(call.head);
        if NuDataFrame::can_downcast(&value) {
            let df = NuDataFrame::try_from_value(value)?;
            command(call, df).map_err(|e| e.into())
        } else {
            let expr = NuExpression::try_from_value(value)?;
            let expr: NuExpression = expr.into_polars().last().into();

            Ok(PipelineData::Value(
                NuExpression::into_value(expr, call.head),
                None,
            ))
        }
    }
}

fn examples() -> Vec<PluginExample> {
    vec![
        PluginExample {
            description: "Create new dataframe with last rows".into(),
            example: "[[a b]; [1 2] [3 4]] | dfr into-df | dfr last 1".into(),
            // result: Some(
            //     NuDataFrame::try_from_columns(
            //         vec![
            //             Column::new("a".to_string(), vec![Value::test_int(3)]),
            //             Column::new("b".to_string(), vec![Value::test_int(4)]),
            //         ],
            //         None,
            //     )
            //     .expect("simple df for test should not fail")
            //     .into_value(Span::test_data()),
            // ),
            result: None,
        },
        PluginExample {
            description: "Creates a last expression from a column".into(),
            example: "dfr col a | dfr last".into(),
            result: None,
        },
    ]
}
fn command(call: &EvaluatedCall, df: NuDataFrame) -> Result<PipelineData, ShellError> {
    let rows: Option<usize> = call.opt(0)?;
    let rows = rows.unwrap_or(DEFAULT_ROWS);

    let res = df.as_ref().tail(Some(rows));
    Ok(PipelineData::Value(
        NuDataFrame::dataframe_into_value(res, call.head)?,
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
