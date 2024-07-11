use crate::{dataframe::values::NuExpression, values::CustomValueSupport, PolarsPlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    record, Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExprLit;

impl PluginCommand for ExprLit {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars lit"
    }

    fn usage(&self) -> &str {
        "Creates a literal expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "literal",
                SyntaxShape::Any,
                "literal to construct the expression",
            )
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Created a literal expression and converts it to a nu object",
            example: "polars lit 2 | polars into-nu",
            result: Some(Value::test_record(record! {
                "expr" =>  Value::test_string("literal"),
                "value" => Value::test_string("dyn int: 2"),
            })),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["string", "literal", "expression"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let literal: Value = call.req(0)?;
        let expr = NuExpression::try_from_value(plugin, &literal)?;
        expr.to_pipeline_data(plugin, engine, call.head)
            .map_err(LabeledError::from)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_polars_plugin_command;

    #[test]
    fn test_examples() -> Result<(), nu_protocol::ShellError> {
        test_polars_plugin_command(&ExprLit)
    }
}
