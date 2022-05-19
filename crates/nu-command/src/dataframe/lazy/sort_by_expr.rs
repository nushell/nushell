use super::super::values::NuLazyFrame;
use crate::dataframe::values::NuExpression;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct LazySortBy;

impl Command for LazySortBy {
    fn name(&self) -> &str {
        "dfr sort-by"
    }

    fn usage(&self) -> &str {
        "sorts a lazy dataframe based on expression(s)"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required(
                "filter expression",
                SyntaxShape::Any,
                "filtering expression",
            )
            .named(
                "reverse",
                SyntaxShape::List(Box::new(SyntaxShape::Boolean)),
                "list indicating if reverse search should be done in the column. Default is false",
                Some('r'),
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

        let reverse: Option<Vec<bool>> = call.get_flag(engine_state, stack, "reverse")?;
        let reverse = match reverse {
            Some(list) => {
                if expressions.len() != list.len() {
                    let span = call
                        .get_flag::<Value>(engine_state, stack, "reverse")?
                        .expect("already checked and it exists")
                        .span()?;
                    return Err(ShellError::GenericError(
                        "Incorrect list size".into(),
                        "Size doesn't match expression list".into(),
                        Some(span),
                        None,
                        Vec::new(),
                    ));
                } else {
                    list
                }
            }
            None => expressions.iter().map(|_| false).collect::<Vec<bool>>(),
        };

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let lazy: NuLazyFrame = lazy
            .into_polars()
            .sort_by_exprs(&expressions, reverse)
            .into();

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
//        test_dataframe(vec![Box::new(LazySortBy {})])
//    }
//}
