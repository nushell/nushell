use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression},
    values::CustomValueSupport,
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

    fn description(&self) -> &str {
        "Creates an expression that returns the arguments where expression is true."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("column name", SyntaxShape::Any, "Expression to evaluate")
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
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
                .into_value(Span::test_data()),
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
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let value: Value = call.req(0)?;
        let expr = NuExpression::try_from_value(plugin, &value)?;
        let expr: NuExpression = arg_where(expr.into_polars()).into();
        expr.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
            .map(|pd| pd.set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&ExprArgWhere)
    }
}
