use crate::dataframe::values::NuExpression;
use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct ExprLit;

impl Command for ExprLit {
    fn name(&self) -> &str {
        "dfr lit"
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
            example: "dfr lit 2 | dfr into-nu",
            result: Some(Value::test_record(record! {
                "expr" =>  Value::test_string("literal"),
                "value" => Value::test_string("2"),
            })),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["string", "literal", "expression"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let literal: Value = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_value(literal)?;
        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::eager::ToNu;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprLit {}), Box::new(ToNu {})])
    }
}
