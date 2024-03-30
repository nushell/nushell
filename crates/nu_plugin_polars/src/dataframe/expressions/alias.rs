use crate::{
    values::{to_pipeline_data, CustomValueSupport},
    PolarsPlugin,
};

use super::super::values::NuExpression;

use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    record, Category, Example, LabeledError, PipelineData, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct ExprAlias;

impl PluginCommand for ExprAlias {
    type Plugin = PolarsPlugin;

    fn name(&self) -> &str {
        "polars as"
    }

    fn usage(&self) -> &str {
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
                Type::Custom("expression".into()),
                Type::Custom("expression".into()),
            )
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
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
        let alias: String = call.req(0)?;

        let expr = NuExpression::try_from_pipeline(plugin, input, call.head)?;
        let expr: NuExpression = expr.to_polars().alias(alias.as_str()).into();

        to_pipeline_data(plugin, engine, call.head, expr).map_err(LabeledError::from)
    }
}

// todo: fix tests
// #[cfg(test)]
// mod test {
//     use super::super::super::test_dataframe::test_dataframe;
//     use super::*;
//     use crate::dataframe::eager::ToNu;
//     use crate::dataframe::expressions::ExprCol;
//
//     #[test]
//     fn test_examples() {
//         test_dataframe(vec![
//             Box::new(ExprAlias {}),
//             Box::new(ExprCol {}),
//             Box::new(ToNu {}),
//         ])
//     }
// }
