use crate::{
    PolarsPlugin,
    dataframe::values::{Column, NuDataFrame, NuExpression, NuWhen},
    values::{CustomValueSupport, NuWhenType},
};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, Span, SyntaxShape, Type, Value,
};
use polars::prelude::when;

#[derive(Clone)]
pub struct ExprWhen;

impl PluginCommand for ExprWhen {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars when"
    }

    fn description(&self) -> &str {
        "Creates and modifies a when expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "when expression",
                SyntaxShape::Any,
                "when expression used for matching",
            )
            .required(
                "then expression",
                SyntaxShape::Any,
                "expression that will be applied when predicate is true",
            )
            .input_output_types(vec![
                (Type::Nothing, Type::Custom("expression".into())),
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                // FIXME Type::Any input added to disable pipeline input type checking, as run-time checks can raise undesirable type errors
                // which aren't caught by the parser. see https://github.com/nushell/nushell/pull/14922 for more details
                (Type::Any, Type::Custom("expression".into())),
            ])
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Create a when conditions",
                example: "polars when ((polars col a) > 2) 4",
                result: None,
            },
            Example {
                description: "Create a when conditions",
                example: "polars when ((polars col a) > 2) 4 | polars when ((polars col a) < 0) 6",
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
        vec!["condition", "match", "if", "else"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let when_predicate: Value = call.req(0)?;
        let when_predicate = NuExpression::try_from_value(plugin, &when_predicate)?;

        let then_predicate: Value = call.req(1)?;
        let then_predicate = NuExpression::try_from_value(plugin, &then_predicate)?;

        let value = input.into_value(call.head)?;
        let when_then: NuWhen = match value {
            Value::Nothing { .. } => when(when_predicate.into_polars())
                .then(then_predicate.into_polars())
                .into(),
            v => match NuWhen::try_from_value(plugin, &v)?.when_type {
                NuWhenType::Then(when_then) => when_then
                    .when(when_predicate.into_polars())
                    .then(then_predicate.into_polars())
                    .into(),
                NuWhenType::ChainedThen(when_then_then) => when_then_then
                    .when(when_predicate.into_polars())
                    .then(then_predicate.into_polars())
                    .into(),
            },
        };

        when_then
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
        test_polars_plugin_command(&ExprWhen)
    }
}
