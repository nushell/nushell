use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape,
};

use super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct ColumnDF;

impl Command for ColumnDF {
    fn name(&self) -> &str {
        "column"
    }

    fn usage(&self) -> &str {
        "Returns the selected column"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("column", SyntaxShape::String, "column name")
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns the selected column as series",
            example: "[[a b]; [1 2] [3 4]] | to df | column a",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "a".to_string(),
                    vec![1.into(), 3.into()],
                )])
                .expect("simple df for test should not fail")
                .into_value(Span::unknown()),
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let column: Spanned<String> = call.req(engine_state, stack, 0)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let res = df.as_ref().column(&column.item).map_err(|e| {
        ShellError::SpannedLabeledError("Error selecting column".into(), e.to_string(), column.span)
    })?;

    NuDataFrame::try_from_series(vec![res.clone()], call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(ColumnDF {})
    }
}
