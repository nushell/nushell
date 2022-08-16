use crate::dataframe::values::{Column, NuDataFrame, NuExpression, NuLazyFrame};

use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};
#[derive(Clone)]
pub struct LazySelect;

impl Command for LazySelect {
    fn name(&self) -> &str {
        "select"
    }

    fn usage(&self) -> &str {
        "Selects columns from lazyframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest(
                "select expressions",
                SyntaxShape::Any,
                "Expression(s) that define the column selection",
            )
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Select a column from the dataframe",
            example: "[[a b]; [6 2] [4 2] [2 2]] | into df | select a",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "a".to_string(),
                    vec![Value::test_int(6), Value::test_int(4), Value::test_int(2)],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::test_data()),
            ),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let vals: Vec<Value> = call.rest(engine_state, stack, 0)?;
        let value = Value::List {
            vals,
            span: call.head,
        };
        let expressions = NuExpression::extract_exprs(value)?;

        let lazy = NuLazyFrame::try_from_pipeline(input, call.head)?;
        let lazy = NuLazyFrame::new(lazy.from_eager, lazy.into_polars().select(&expressions));

        Ok(PipelineData::Value(lazy.into_value(call.head)?, None))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(LazySelect {})])
    }
}
