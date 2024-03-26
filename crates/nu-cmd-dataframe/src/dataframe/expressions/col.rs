use crate::dataframe::values::NuExpression;
use nu_engine::command_prelude::*;

use polars::prelude::col;

#[derive(Clone)]
pub struct ExprCol;

impl Command for ExprCol {
    fn name(&self) -> &str {
        "dfr col"
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
            example: "dfr col a | dfr into-nu",
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let name: String = call.req(engine_state, stack, 0)?;
        let expr: NuExpression = col(name.as_str()).into();

        Ok(PipelineData::Value(expr.into_value(call.head), None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;
    use crate::dataframe::eager::ToNu;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprCol {}), Box::new(ToNu {})])
    }
}
