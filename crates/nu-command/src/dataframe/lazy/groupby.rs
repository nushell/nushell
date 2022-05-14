use super::into_expression::IntoExpression;
use crate::dataframe::values::{NuLazyFrame, NuLazyGroupBy};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use polars::prelude::Expr;

#[derive(Clone)]
pub struct ToLazyGroupBy;

impl Command for ToLazyGroupBy {
    fn name(&self) -> &str {
        "dfr group-by"
    }

    fn usage(&self) -> &str {
        "Creates a groupby object that can be used for other aggregations"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Group by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the lazy group by",
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
        let expressions = value.into_expressions()?;

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
        let group_by: NuLazyGroupBy = lazy.groupby(&expressions).into();

        Ok(PipelineData::Value(group_by.into_value(call.head), None))
    }
}

//#[cfg(test)]
//mod test {
//    use super::super::super::test_dataframe::test_dataframe;
//    use super::*;
//
//    #[test]
//    fn test_examples() {
//        test_dataframe(vec![Box::new(ToLazyGroupBy {})])
//    }
//}
