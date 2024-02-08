use crate::dataframe::values::str_to_dtype;

use super::super::values::NuDataFrame;
use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct CastDF;

impl Command for CastDF {
    fn name(&self) -> &str {
        "dfr cast"
    }

    fn usage(&self) -> &str {
        "Cast a column to a different dtype."
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_type(
                Type::Custom("dataframe".into()),
                Type::Custom("dataframe".into()),
            )
            .required("column", SyntaxShape::String, "The column to cast")
            .required(
                "dtype",
                SyntaxShape::String,
                "The dtype to cast the column to",
            )
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        //todo: lazy version
        command_eager(engine_state, stack, call, input)
    }
}

fn command_eager(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let column_nm: String = call.req(engine_state, stack, 0)?;
    let dtype: String = call.req(engine_state, stack, 1)?;
    let dtype = str_to_dtype(&dtype, call.head)?;

    let new_df = NuDataFrame::try_from_pipeline(input, call.head)?;
    let mut df = new_df.df;
    let column = df
        .column(&column_nm)
        .map_err(|e| ShellError::GenericError {
            error: format!("{e}"),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let casted = column.cast(&dtype).map_err(|e| ShellError::GenericError {
        error: format!("{e}"),
        msg: "".into(),
        span: Some(call.head),
        help: None,
        inner: vec![],
    })?;

    let _ = df
        .with_column(casted)
        .map_err(|e| ShellError::GenericError {
            error: format!("{e}"),
            msg: "".into(),
            span: Some(call.head),
            help: None,
            inner: vec![],
        })?;

    let df = NuDataFrame::new(false, df);
    Ok(PipelineData::Value(df.into_value(call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(CastDF {})])
    }
}
