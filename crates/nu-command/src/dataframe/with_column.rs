use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned, SyntaxShape, Value,
};

use super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct WithColumn;

impl Command for WithColumn {
    fn name(&self) -> &str {
        "dfr with-column"
    }

    fn usage(&self) -> &str {
        "Adds a series to the dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("series", SyntaxShape::Any, "series to be added")
            .required_named("name", SyntaxShape::String, "column name", Some('n'))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Adds a series to the dataframe",
            example:
                "[[a b]; [1 2] [3 4]] | dfr to-df | dfr with-column ([5 6] | dfr to-df) --name c",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "a".to_string(),
                        vec![Value::test_int(1), Value::test_int(3)],
                    ),
                    Column::new(
                        "b".to_string(),
                        vec![Value::test_int(2), Value::test_int(4)],
                    ),
                    Column::new(
                        "c".to_string(),
                        vec![Value::test_int(5), Value::test_int(6)],
                    ),
                ])
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let name: Spanned<String> = call
        .get_flag(engine_state, stack, "name")?
        .expect("required named value");

    let other_value: Value = call.req(engine_state, stack, 0)?;
    let other_span = other_value.span()?;
    let mut other = NuDataFrame::try_from_value(other_value)?.as_series(other_span)?;
    let series = other.rename(&name.item).clone();

    let mut df = NuDataFrame::try_from_pipeline(input, call.head)?;

    df.as_mut()
        .with_column(series)
        .map_err(|e| {
            ShellError::SpannedLabeledError(
                "Error adding column to dataframe".into(),
                e.to_string(),
                other_span,
            )
        })
        .map(|df| {
            PipelineData::Value(
                NuDataFrame::dataframe_into_value(df.clone(), call.head),
                None,
            )
        })
}

#[cfg(test)]
mod test {
    use super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(WithColumn {})])
    }
}
