use super::super::super::values::{Column, NuDataFrame};

use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};
use polars::prelude::{arg_where, col, IntoLazy};

#[derive(Clone)]
pub struct ArgTrue;

impl Command for ArgTrue {
    fn name(&self) -> &str {
        "arg-true"
    }

    fn usage(&self) -> &str {
        "Returns indexes where values are true"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argtrue", "truth", "boolean-true"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes where values are true",
            example: "[false true false] | into df | arg-true",
            result: Some(
                NuDataFrame::try_from_columns(vec![Column::new(
                    "arg_true".to_string(),
                    vec![Value::test_int(1)],
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let columns = df.as_ref().get_column_names();
    if columns.len() > 1 {
        return Err(ShellError::GenericError(
            "Error using as series".into(),
            "dataframe has more than one column".into(),
            Some(call.head),
            None,
            Vec::new(),
        ));
    }

    match columns.first() {
        Some(column) => {
            let expression = arg_where(col(column).eq(true)).alias("arg_true");
            let res = df
                .as_ref()
                .clone()
                .lazy()
                .select(&[expression])
                .collect()
                .map_err(|err| {
                    ShellError::GenericError(
                        "Error creating index column".into(),
                        err.to_string(),
                        Some(call.head),
                        None,
                        Vec::new(),
                    )
                })?;

            let value = NuDataFrame::dataframe_into_value(res, call.head);
            Ok(PipelineData::Value(value, None))
        }
        _ => todo!(),
    }
}

#[cfg(test)]
mod test {
    use super::super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ArgTrue {})])
    }
}
