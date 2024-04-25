use crate::{dataframe::values::NuExpression, values::CustomValueSupport, PolarsPlugin};
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    record, Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};
use polars::prelude::col;

#[derive(Clone)]
pub struct ExprCol;

impl PluginCommand for ExprCol {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars col"
    }

    fn usage(&self) -> &str {
        "Creates a named column expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "column name",
                SyntaxShape::String,
                "Name of column to be used",
            )
            .input_output_type(Type::Any, Type::Custom("expression".into()))
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named column expression and converts it to a nu object",
            example: "polars col a | polars into-nu",
            result: Some(Value::test_record(record! {
                "expr" =>  Value::test_string("column"),
                "value" => Value::test_string("a"),
            })),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["create"]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        let name: String = call.req(0)?;
        let expr: NuExpression = col(name.as_str()).into();
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
        test_polars_plugin_command(&ExprCol)
    }
}
