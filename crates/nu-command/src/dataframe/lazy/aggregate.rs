use crate::dataframe::values::{NuLazyFrame, NuLazyGroupBy};

use super::into_expression::IntoExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LazyAggregate;

impl Command for LazyAggregate {
    fn name(&self) -> &str {
        "dfl aggregate"
    }

    fn usage(&self) -> &str {
        "Performs a series of aggregations from a lazy group by"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "Group by expressions",
                SyntaxShape::Any,
                "Expression(s) that define the aggregations to be applied",
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

        let group_by = NuLazyGroupBy::try_from_pipeline(input, call.head)?.into_polars();
        let lazy: NuLazyFrame = group_by.agg(&expressions).into();

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
//        test_dataframe(vec![Box::new(LazyAggregate {})])
//    }
//}
