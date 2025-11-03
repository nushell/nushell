use crate::{PolarsPlugin, values::CustomValueSupport};

use crate::values::{NuExpression, PolarsPluginType};

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Value, record,
};

#[derive(Clone)]
pub struct ExprAlias;

impl PluginCommand for ExprAlias {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars as"
    }

    fn description(&self) -> &str {
        "Creates an alias expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Alias name",
                SyntaxShape::String,
                "Alias name for the expression",
            )
            .input_output_type(
                PolarsPluginType::NuExpression.into(),
                PolarsPluginType::NuExpression.into(),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Creates and alias expression",
            example: "polars col a | polars as new_a | polars into-nu",
            result: {
                let record = Value::test_record(record! {
                    "expr" =>  Value::test_record(record! {
                        "expr" =>  Value::test_string("column"),
                        "value" => Value::test_string("a"),
                    }),
                    "alias" => Value::test_string("new_a"),
                });

                Some(record)
            },
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["aka", "abbr", "otherwise"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let metadata = input.metadata();
        let alias: String = call.req(0)?;

        let expr = NuExpression::try_from_pipeline(plugin, input, call.head)?;
        let expr: NuExpression = expr.into_polars().alias(alias.as_str()).into();

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
        test_polars_plugin_command(&ExprAlias)
    }
}
