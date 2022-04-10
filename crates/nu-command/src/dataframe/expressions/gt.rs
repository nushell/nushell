use super::super::values::NuExpression;

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::Expr;

#[derive(Clone)]
pub struct ExprGt;

impl Command for ExprGt {
    fn name(&self) -> &str {
        "expr gt"
    }

    fn usage(&self) -> &str {
        "Creates a greater than expression"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Comparing expression",
                SyntaxShape::Any,
                "Expression to compare against",
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
        let literal: Value = call.req(engine_state, stack, 0)?;
        let literal = NuExpression::try_from_value(literal)?;
        let expr = NuExpression::try_from_pipeline(input, call.head)?;
        let expr = expr.apply_with_expr(literal, Expr::gt);

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
//        test_dataframe(vec![Box::new(ExprGt {})])
//    }
//}
