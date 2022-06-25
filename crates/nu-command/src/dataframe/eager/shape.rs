use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Type, Value,
};

use crate::dataframe::values::Column;

use super::super::values::NuDataFrame;

#[derive(Clone)]
pub struct ShapeDF;

impl Command for ShapeDF {
    fn name(&self) -> &str {
        "shape"
    }

    fn usage(&self) -> &str {
        "Shows column and row size for a dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_type(Type::Custom("dataframe".into()))
            .output_type(Type::Custom("dataframe".into()))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Shows row and column shape",
            example: "[[a b]; [1 2] [3 4]] | into df | shape",
            result: Some(
                NuDataFrame::try_from_columns(vec![
                    Column::new("rows".to_string(), vec![Value::test_int(2)]),
                    Column::new("columns".to_string(), vec![Value::test_int(2)]),
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    let rows = Value::Int {
        val: df.as_ref().height() as i64,
        span: call.head,
    };

    let cols = Value::Int {
        val: df.as_ref().width() as i64,
        span: call.head,
    };

    let rows_col = Column::new("rows".to_string(), vec![rows]);
    let cols_col = Column::new("columns".to_string(), vec![cols]);

    NuDataFrame::try_from_columns(vec![rows_col, cols_col])
        .map(|df| PipelineData::Value(df.into_value(call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(ShapeDF {})])
    }
}
