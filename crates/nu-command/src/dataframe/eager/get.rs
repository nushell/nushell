use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Type, Value,
};

use crate::dataframe::values::utils::convert_columns_string;

use super::super::values::{Column, NuDataFrame};

#[derive(Clone)]
pub struct GetDF;

impl Command for GetDF {
    fn name(&self) -> &str {
        "get"
    }

    fn usage(&self) -> &str {
        "Creates dataframe with the selected columns"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .rest("rest", SyntaxShape::Any, "column names to sort dataframe")
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns the selected column",
            example: "[[a b]; [1 2] [3 4]] | into df | get a",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "a".to_string(),
                    vec![Value::test_int(1), Value::test_int(3)],
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
        command(engine_state, stack, call, input)
    }
}

fn command(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let columns: Vec<Value> = call.rest(engine_state, stack, 0)?;
    let (col_string, col_span) = convert_columns_string(columns, call.head)?;

    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    df.as_ref()
        .select(&col_string)
        .map_err(|e| {
            ShellError::GenericError(
                "Error selecting columns".into(),
                e.to_string(),
                Some(col_span),
                None,
                Vec::new(),
            )
        })
        .map(|df| PipelineData::Value(NuDataFrame::dataframe_into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(GetDF {})])
    }
}
