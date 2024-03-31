use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Span, SyntaxShape, Type,
    Value,
};
use polars::prelude::{lit, DataType};

#[derive(Clone)]
pub struct ExprIsIn;

impl PluginCommand for ExprIsIn {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars is-in"
    }

    fn usage(&self) -> &str {
        "Creates an is-in expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "list",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "List to check if values are in",
            )
            .input_output_type(
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a is-in expression",
            example: r#"let df = ([[a b]; [one 1] [two 2] [three 3]] | polars into-df);
    $df | polars with-column (polars col a | polars is-in [one two] | polars as a_in)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![
                                Value::test_string("one"),
                                Value::test_string("two"),
                                Value::test_string("three"),
                            ],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_int(1), Value::test_int(2), Value::test_int(3)],
                        ),
                        Column::new(
                            "a_in".to_string(),
                            vec![
                                Value::test_bool(true),
                                Value::test_bool(true),
                                Value::test_bool(false),
                            ],
                        ),
                    ],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
            ),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["check", "contained", "is-contain", "match"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let list: Vec<Value> = call.req(0)?;
        let expr = NuExpression::try_from_pipeline(plugin, input, call.head)?;

        let values =
            NuDataFrame::try_from_columns(vec![Column::new("list".to_string(), list)], None)?;
        let list = values.as_series(call.head)?;

        if matches!(list.dtype(), DataType::Object(..)) {
            return Err(LabeledError::from(
                ShellError::IncompatibleParametersSingle {
                    msg: "Cannot use a mixed list as argument".into(),
                    span: call.head,
                },
            ));
        }

        let expr: NuExpression = expr.to_polars().is_in(lit(list)).into();
        to_pipeline_data(plugin, engine, call.head, expr).map_err(LabeledError::from)
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::eager::WithColumn;
//     use crate::dataframe::expressions::alias::ExprAlias;
//     use crate::dataframe::expressions::col::ExprCol;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![
//             Box::new(ExprIsIn {}),
//             Box::new(ExprAlias {}),
//             Box::new(ExprCol {}),
//             Box::new(WithColumn {}),
//         ])
//     }
// }
