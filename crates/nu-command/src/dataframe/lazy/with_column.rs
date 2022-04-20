use super::super::values::NuLazyFrame;
use super::into_expression::IntoExpression;
use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::LazyFrame;

#[derive(Clone)]
pub struct LazyWithColumn;

impl Command for LazyWithColumn {
    fn name(&self) -> &str {
        "dfl with-column"
    }

    fn usage(&self) -> &str {
        "Adds new column(s) for the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "expression(s)",
                SyntaxShape::Any,
                "Expression(s) that will create the new column(s)",
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
        let value_span = value.span()?;
        let expr = NuExpression::try_from_value(value.clone());
        let expressions = value.into_expressions();

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;

        let lazy: NuLazyFrame = match (expr, expressions) {
            (Ok(expr), Err(_)) => lazy.apply_with_expr(expr, LazyFrame::with_column),
            (Err(_), Ok(expressions)) => lazy.into_polars().with_columns(&expressions).into(),
            _ => {
                return Err(ShellError::IncompatibleParametersSingle(
                    "Expected only a expression or list of expressions".into(),
                    value_span,
                ));
            }
        };

        Ok(PipelineData::Value(
            NuLazyFrame::into_value(lazy, call.head),
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
//        test_dataframe(vec![Box::new(LazyWithColumn {})])
//    }
//}
