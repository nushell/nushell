use crate::dataframe::values::{NuExpression, NuLazyFrame};
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LazyShift;

impl Command for LazyShift {
    fn name(&self) -> &str {
        "dfl shift"
    }

    fn usage(&self) -> &str {
        "Shifts lazy frame values by a given period"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "shift",
                SyntaxShape::Int,
                "Number of values to shift the lazyframe",
            )
            .named(
                "fill",
                SyntaxShape::Any,
                "Expression to use to fill the null values",
                Some('f'),
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
        let shift: i64 = call.req(engine_state, stack, 0)?;
        let fill: Option<Value> = call.get_flag(engine_state, stack, "fill")?;

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?.into_polars();

        let lazy: NuLazyFrame = match fill {
            Some(fill) => {
                let expr = NuExpression::try_from_value(fill)?.into_polars();
                lazy.shift_and_fill(shift, expr).into()
            }
            None => lazy.shift(shift).into(),
        };

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
//        test_dataframe(vec![Box::new(LazyShift {})])
//    }
//}
