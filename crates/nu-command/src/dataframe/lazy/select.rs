use crate::dataframe::values::{NuExpression, NuLazyFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::Expr;

#[derive(Clone)]
pub struct LazySelect;

impl Command for LazySelect {
    fn name(&self) -> &str {
        "dfr select"
    }

    fn usage(&self) -> &str {
        "Selects columns from lazyframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "select expressions",
                SyntaxShape::Any,
                "Expression(s) that define the column selection",
            )
            .category(Category::Custom("lazyframe".into()))
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
        let value: Value = call.req(engine_state, stack, 0)?;
        let expressions = NuExpression::extract_exprs(value)?;

        if expressions
            .iter()
            .any(|expr| !matches!(expr, Expr::Column(..)))
        {
            let value: Value = call.req(engine_state, stack, 0)?;
            return Err(ShellError::IncompatibleParametersSingle(
                "Expected only Col expressions".into(),
                value.span()?,
            ));
        }

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();
        let lazy: NuLazyFrame = lazy.select(&expressions).into();

        Ok(PipelineData::Value(lazy.into_value(call.head), None))
    }
}

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(LazySelect {})])
//    }
//}
