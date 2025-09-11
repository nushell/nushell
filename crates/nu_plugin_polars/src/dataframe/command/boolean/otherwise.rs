use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression, NuWhen, NuWhenType},
    values::CustomValueSupport,
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExprOtherwise;

impl PluginCommand for ExprOtherwise {
    type Plugin = PolarsPlugin;
    fn name(&self) -> &str {
        "polars otherwise"
    }

    fn description(&self) -> &str {
        "Completes a when expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "otherwise expression",
                SyntaxShape::Any,
                "expression to apply when no when predicate matches",
            )
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a when conditions",
                example: "polars when ((polars col a) > 2) 4 | polars otherwise 5",
                result: None,
            },
            Example {
                description: "Create a when conditions",
                example: "polars when ((polars col a) > 2) 4 | polars when ((polars col a) < 0) 6 | polars otherwise 0",
                result: None,
            },
            Example {
                description: "Create a new column for the dataframe",
                example: r#"[[a b]; [6 2] [1 4] [4 1]]
   | polars into-lazy
   | polars with-column (
    polars when ((polars col a) > 2) 4 | polars otherwise 5 | polars as c
     )
   | polars with-column (
    polars when ((polars col a) > 5) 10 | polars when ((polars col a) < 2) 6 | polars otherwise 0 | polars as d
     )
   | polars collect"#,
                result: Some(
                    NuDataFrame::try_from_columns(
                        vec![
                            Column::new(
                                "a".to_string(),
                                vec![Value::test_int(6), Value::test_int(1), Value::test_int(4)],
                            ),
                            Column::new(
                                "b".to_string(),
                                vec![Value::test_int(2), Value::test_int(4), Value::test_int(1)],
                            ),
                            Column::new(
                                "c".to_string(),
                                vec![Value::test_int(4), Value::test_int(5), Value::test_int(4)],
                            ),
                            Column::new(
                                "d".to_string(),
                                vec![Value::test_int(10), Value::test_int(6), Value::test_int(0)],
                            ),
                        ],
                        None,
                    )
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["condition", "else"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let otherwise_predicate: Value = call.req(0)?;
        let otherwise_predicate = NuExpression::try_from_value(plugin, &otherwise_predicate)?;

        let value = input.into_value(call.head)?;
        let complete: NuExpression = match NuWhen::try_from_value(plugin, &value)?.when_type {
            NuWhenType::Then(then) => then.otherwise(otherwise_predicate.into_polars()).into(),
            NuWhenType::ChainedThen(chained_when) => chained_when
                .otherwise(otherwise_predicate.into_polars())
                .into(),
        };
        complete
            .to_pipeline_data(plugin, engine, call.head)
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
        test_polars_plugin_command(&ExprOtherwise)
    }
}
