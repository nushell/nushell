use nu_engine::CallExt;
use nu_protocol::ast::{Call, CellPath};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData,
    ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Reject;

impl Command for Reject {
    fn name(&self) -> &str {
        "reject"
    }

    fn signature(&self) -> Signature {
        Signature::build("reject")
            .rest(
                "rest",
                SyntaxShape::CellPath,
                "the names of columns to remove from the table",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Remove the given columns from the table. If you want to remove rows, try 'drop'."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let columns: Vec<CellPath> = call.rest(engine_state, stack, 0)?;
        let span = call.head;
        reject(engine_state, span, input, columns)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Lists the files in a directory without showing the modified column",
                example: "ls | reject modified",
                result: None,
            },
            Example {
                description: "Reject the specified field in a record",
                example: "echo {a: 1, b: 2} | reject a",
                result: Some(Value::Record {
                    cols: vec!["b".into()],
                    vals: vec![Value::Int {
                        val: 2,
                        span: Span::test_data(),
                    }],
                    span: Span::test_data(),
                }),
            },
        ]
    }
}

fn reject(
    _engine_state: &EngineState,
    span: Span,
    input: PipelineData,
    columns: Vec<CellPath>,
) -> Result<PipelineData, ShellError> {
    let val = input.into_value(span);
    let mut val = val;
    for cell_path in columns {
        val.remove_data_at_cell_path(&cell_path.members)?;
    }
    Ok(val.into_pipeline_data())
}

#[cfg(test)]
mod test {
    #[test]
    fn test_examples() {
        use super::Reject;
        use crate::test_examples;
        test_examples(Reject {})
    }
}
