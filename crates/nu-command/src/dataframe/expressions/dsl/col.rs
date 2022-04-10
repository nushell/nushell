use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::{col, cols};

#[derive(Clone)]
pub struct ExprCol;

impl Command for ExprCol {
    fn name(&self) -> &str {
        "expr col"
    }

    fn usage(&self) -> &str {
        "Creates a named column expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "column(s) name",
                SyntaxShape::Any,
                "Name of column(s) to be used",
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Creates a named column expression",
            example: "expr col col_a",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let names: Value = call.req(engine_state, stack, 0)?;

        let expr = match names {
            Value::String { val, .. } => Ok(NuExpression::new(col(val.as_str()))),
            Value::List { vals, .. } => vals
                .iter()
                .map(|val| val.as_string())
                .collect::<Result<Vec<String>, ShellError>>()
                .map(|names| NuExpression::new(cols(names))),
            _ => Err(ShellError::SpannedLabeledError(
                "Incorrect type for columns".into(),
                "Expected string or list of strings".into(),
                names.span()?,
            )),
        }?;

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(ExprCol {})])
//    }
//}
