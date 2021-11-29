use super::super::{Column, NuDataFrame};
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Value,
};

#[derive(Clone)]
pub struct DataTypes;

impl Command for DataTypes {
    fn name(&self) -> &str {
        "dtypes"
    }

    fn usage(&self) -> &str {
        "Show dataframe data types"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name().to_string()).category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "drop column a",
            example: "[[a b]; [1 2] [3 4]] | to df | dtypes",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new(
                        "column".to_string(),
                        vec!["a".to_string().into(), "b".to_string().into()],
                    ),
                    Column::new(
                        "dtype".to_string(),
                        vec!["i64".to_string().into(), "i64".to_string().into()],
                    ),
                ])
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

#[allow(clippy::needless_collect)]
fn command(
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head.clone())?;

    let mut dtypes: Vec<Value> = Vec::new();
    let names: Vec<Value> = df
        .as_ref()
        .get_column_names()
        .iter()
        .map(|v| {
            let dtype = df
                .as_ref()
                .column(v)
                .expect("using name from list of names from dataframe")
                .dtype();

            let dtype_str = dtype.to_string();
            dtypes.push(Value::String {
                val: dtype_str.into(),
                span: call.head,
            });

            Value::String {
                val: v.to_string().into(),
                span: call.head,
            }
        })
        .collect();

    let names_col = Column::new("column".to_string(), names);
    let dtypes_col = Column::new("dtype".to_string(), dtypes);

    let df = NuDataFrame::try_from_columns(vec![names_col, dtypes_col])?;
    Ok(PipelineData::Value(df.into_value(call.head)))
}

#[cfg(test)]
mod test {
    use super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(DataTypes {})
    }
}
