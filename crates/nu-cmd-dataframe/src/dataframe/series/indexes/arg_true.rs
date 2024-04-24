use crate::dataframe::values::{Column, NuDataFrame};
use nu_engine::command_prelude::*;
use polars::prelude::{arg_where, col, IntoLazy};

#[derive(Clone)]
pub struct ArgTrue;

impl Command for ArgTrue {
    fn name(&self) -> &str {
        "dfr arg-true"
    }

    fn usage(&self) -> &str {
        "Returns indexes where values are true."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["argtrue", "truth", "boolean-true"]
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Returns indexes where values are true",
            example: "[false true false] | dfr into-df | dfr arg-true",
            result: Some(
                NuDataFrame::try_from_columns(
                    vec![Column::new(
                        "arg_true".to_string(),
                        vec![Value::test_int(1)],
                    )],
                    None,
                )
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
        return Err(ShellError::GenericError {
            error: "Error using as series".into(),
            msg: "dataframe has more than one column".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        });
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
                .map_err(|err| ShellError::GenericError {
                    error: "Error creating index column".into(),
                    msg: err.to_string(),
                    span: Some(call.head),
                    help: None,
                    inner: vec![],
                })?;

            let value = NuDataFrame::dataframe_into_value(res, call.head);
            Ok(PipelineData::Value(value, None))
        }
        _ => Err(ShellError::UnsupportedInput {
            msg: "Expected the dataframe to have a column".to_string(),
            input: "".to_string(),
            msg_span: call.head,
            input_span: call.head,
        }),
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
