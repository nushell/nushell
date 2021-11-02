use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, Signature, SyntaxShape,
    Value,
};

#[derive(Clone)]
pub struct Zip;

impl Command for Zip {
    fn name(&self) -> &str {
        "zip"
    }

    fn usage(&self) -> &str {
        "Combine a stream with the input"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("zip").required("other", SyntaxShape::Any, "the other input")
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "1..3 | zip 4..6",
            description: "Zip multiple streams and get one of the results",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let other: Value = call.req(engine_state, stack, 0)?;
        let head = call.head;
        let ctrlc = engine_state.ctrlc.clone();

        Ok(input
            .into_iter()
            .zip(other.into_pipeline_data().into_iter())
            .map(move |(x, y)| Value::List {
                vals: vec![x, y],
                span: head,
            })
            .into_pipeline_data(ctrlc))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Zip {})
    }
}
