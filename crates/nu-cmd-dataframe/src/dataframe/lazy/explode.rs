use crate::dataframe::values::{NuDataFrame, NuExpression, NuLazyFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct LazyExplode;

impl Command for LazyExplode {
    fn name(&self) -> &str {
        "dfr explode"
    }

    fn usage(&self) -> &str {
        "Explods a dataframe or creates a explode expression."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .optional(
                "columns",
                SyntaxShape::String,
                "columns to explode, only applicable for dataframes",
            )
            .input_output_types(vec![
                (
                    Type::Custom("expression".into()),
                    Type::Custom("expression".into()),
                ),
                (
                    Type::Custom("dataframe".into()),
                    Type::Custom("dataframe".into()),
                ),
            ])
            .category(Category::Custom("lazyframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Explode the specified dataframe",
                example: "",
                result: None,
            },
            Example {
                description: "todo expression case",
                example: "",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        explode(call, input)
    }
}

pub(crate) fn explode(call: &Call, input: PipelineData) -> Result<PipelineData, ShellError> {
    let value = input.into_value(call.head);
    if NuDataFrame::can_downcast(&value) {
        let df = NuLazyFrame::try_from_value(value)?;
        let columns: Vec<String> = call
            .positional_iter()
            .filter_map(|e| e.as_string())
            .collect();

        let exploded = df
            .into_polars()
            .explode(columns.iter().map(AsRef::as_ref).collect::<Vec<&str>>());

        Ok(PipelineData::Value(
            NuLazyFrame::from(exploded).into_value(call.head)?,
            None,
        ))
    } else {
        let expr = NuExpression::try_from_value(value)?;
        let expr: NuExpression = expr.into_polars().explode().into();

        Ok(PipelineData::Value(
            NuExpression::into_value(expr, call.head),
            None,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::{build_test_engine_state, test_dataframe_example};
    use super::*;
    use crate::dataframe::lazy::aggregate::LazyAggregate;
    use crate::dataframe::lazy::groupby::ToLazyGroupBy;

    #[test]
    fn test_examples_dataframe() {
        let mut engine_state = build_test_engine_state(vec![Box::new(LazyExplode {})]);
        test_dataframe_example(&mut engine_state, &LazyExplode.examples()[0]);
        test_dataframe_example(&mut engine_state, &LazyExplode.examples()[1]);
    }

    #[test]
    fn test_examples_expression() {
        let mut engine_state = build_test_engine_state(vec![
            Box::new(LazyExplode {}),
            Box::new(LazyAggregate {}),
            Box::new(ToLazyGroupBy {}),
        ]);
        test_dataframe_example(&mut engine_state, &LazyExplode.examples()[2]);
    }
}
