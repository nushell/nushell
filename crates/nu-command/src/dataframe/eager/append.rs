use nu_engine::CallExt;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

use super::super::values::{Axis, Column, NuDataFrame};

#[derive(Clone)]
pub struct AppendDF;

impl Command for AppendDF {
    fn name(&self) -> &str {
        "dfr append"
    }

    fn usage(&self) -> &str {
        "Appends a new dataframe"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .required("other", SyntaxShape::Any, "dataframe to be appended")
            .switch("col", "appends in col orientation", Some('c'))
            .category(Category::Custom("dataframe".into()))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Appends a dataframe as new columns",
                example: r#"let a = ([[a b]; [1 2] [3 4]] | dfr to-df);
    $a | dfr append $a"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new("a".to_string(), vec![Value::Int(1), Value::Int(3)]),
                        Column::new("b".to_string(), vec![Value::Int(2), Value::Int(4)]),
                        Column::new("a_x".to_string(), vec![Value::Int(1), Value::Int(3)]),
                        Column::new("b_x".to_string(), vec![Value::Int(2), Value::Int(4)]),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
            Example {
                description: "Appends a dataframe merging at the end of columns",
                example: r#"let a = ([[a b]; [1 2] [3 4]] | dfr to-df);
    $a | dfr append $a --col"#,
                result: Some(
                    NuDataFrame::try_from_columns(vec![
                        Column::new(
                            "a".to_string(),
                            vec![Value::Int(1), Value::Int(3), Value::Int(1), Value::Int(3)],
                        ),
                        Column::new(
                            "b".to_string(),
                            vec![Value::Int(2), Value::Int(4), Value::Int(2), Value::Int(4)],
                        ),
                    ])
                    .expect("simple df for test should not fail")
                    .into_value(Span::test_data()),
                ),
            },
        ]
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
    let other: Value = call.req(engine_state, stack, 0)?;

    let axis = if call.has_flag("col") {
        Axis::Column
    } else {
        Axis::Row
    };
    let df_other = NuDataFrame::try_from_value(other)?;
    let df = NuDataFrame::try_from_pipeline(input, call.head)?;

    df.append_df(&df_other, axis, call.head)
        .map(|df| PipelineData::Value(NuDataFrame::into_value(df, call.head), None))
}

#[cfg(test)]
mod test {
    use super::super::super::test_dataframe::test_dataframe;
    use super::*;

    #[test]
    fn test_examples() {
        test_dataframe(vec![Box::new(AppendDF {})])
    }
}
