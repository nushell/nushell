use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::concat_str;

#[derive(Clone)]
pub struct ExprConcatStr;

impl PluginCommand for ExprConcatStr {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars concat-str"
    }

    fn usage(&self) -> &str {
        "Creates a concat string expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "separator",
                SyntaxShape::String,
                "Separator used during the concatenation",
            )
            .required(
                "concat expressions",
                SyntaxShape::List(Box::new(SyntaxShape::Any)),
                "Expression(s) that define the string concatenation",
            )
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a concat string expression",
            example: r#"let df = ([[a b c]; [one two 1] [three four 2]] | polars into-df);
    $df | polars with-column ((polars concat-str "-" [(polars col a) (polars col b) ((polars col c) * 2)]) | polars as concat)"#,
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::test_string("one"), Value::test_string("three")],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::test_string("two"), Value::test_string("four")],
                        ),
                        Column::new(
                            "c".to_string(),
                            vec![Value::test_int(1), Value::test_int(2)],
                        ),
                        Column::new(
                            "concat".to_string(),
                            vec![
                                Value::test_string("one-two-2"),
                                Value::test_string("three-four-4"),
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
        vec!["join", "connect", "update"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let separator: String = call.req(0)?;
        let value: Value = call.req(1)?;

        let expressions = NuExpression::extract_exprs(plugin, value)?;
        let expr: NuExpression = concat_str(expressions, &separator, false).into();

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
//             Box::new(ExprConcatStr {}),
//             Box::new(ExprAlias {}),
//             Box::new(ExprCol {}),
//             Box::new(WithColumn {}),
//         ])
//     }
// }
