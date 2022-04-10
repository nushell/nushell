use super::super::values::NuExpression;

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature,
};

#[derive(Clone)]
pub struct ExprToNu;

impl Command for ExprToNu {
    fn name(&self) -> &str {
        "expr to_nu"
    }

    fn usage(&self) -> &str {
        "Convert expression to a nu value"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "",
            example: "",
            result: None,
        }]
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

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(ExprToNu {})])
//    }
//}
