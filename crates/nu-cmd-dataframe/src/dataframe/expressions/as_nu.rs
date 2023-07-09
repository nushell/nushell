use super::super::values::NuExpression;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, Record, ShellError, Signature, Type, Value,
};

#[derive(Clone)]
pub struct ExprAsNu;

impl Command for ExprAsNu {
    fn name(&self) -> &str {
        "dfr into-nu"
    }

    fn usage(&self) -> &str {
        "Convert expression into a nu value for access and exploration."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(Type::Custom("expression".into()), Type::Any)
            .category(Category::Custom("expression".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Convert a col expression into a nushell value",
            example: "dfr col a | dfr into-nu",
            result: Some(Value::test_record(Record {
                cols: vec!["expr".into(), "value".into()],
                vals: vec![Value::test_string("column"), Value::test_string("a")],
            })),
        }]
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "conversion"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let value = expr.to_value(call.head);

        Ok(PipelineData::Value(value, None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::super::ExprCol;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ExprAsNu {}), Box::new(ExprCol {})])
    }
}
