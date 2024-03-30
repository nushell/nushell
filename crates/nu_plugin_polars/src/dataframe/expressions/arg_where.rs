use crate::{
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::arg_where;

#[derive(Clone)]
pub struct ExprArgWhere;

impl PluginCommand for ExprArgWhere {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars arg-where"
    }

    fn usage(&self) -> &str {
        "Creates an expression that returns the arguments where expression is true."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("column name", SyntaxShape::Any, "Expression to evaluate")
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Return a dataframe where the value match the expression",
            example: "let df = ([[a b]; [one 1] [two 2] [three 3]] | polars into-df);
    $df | polars select (polars arg-where ((polars col b) >= 2) | polars as b_arg)",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "b_arg".to_string(),
                        vec![Value::test_int(1), Value::test_int(2)],
                    )],
                    None,
                )
                .expect("simple df for test should not fail")
                .base_value(Span::test_data())
                .expect("rendering base value should not fail"),
            ),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["condition", "match", "if"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let value: Value = call.req(0)?;
        let expr = NuExpression::try_from_value(plugin, &value)?;
        let expr: NuExpression = arg_where(expr.to_polars()).into();
        to_pipeline_data(plugin, engine, call.head, expr).map_err(LabeledError::from)
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::expressions::ExprAlias;
//     use crate::dataframe::lazy::LazySelect;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![
//             Box::new(ExprArgWhere {}),
//             Box::new(ExprAlias {}),
//             Box::new(LazySelect {}),
//         ])
//     }
// }
