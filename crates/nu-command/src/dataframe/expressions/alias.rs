use super::super::values::NuExpression;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape,
};

#[derive(Clone)]
pub struct ExprAlias;

impl Command for ExprAlias {
    fn name(&self) -> &str {
        "expr alias"
    }

    fn usage(&self) -> &str {
        "Creates an alias expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Alias name",
                SyntaxShape::String,
                "Alias name for the expression",
            )
            .category(Category::Custom("dataframe".into()))
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let alias: String = call.req(engine_state, stack, 0)?;

        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let expr = expr.into_polars().alias(alias.as_str());
        let expr = NuExpression::new(expr);

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
//        test_dataframe(vec![Box::new(ExprAlias {})])
//    }
//}
