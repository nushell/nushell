use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, PipelineData, ShellError, Signature, Span,
    Value,
};

#[derive(Clone)]
pub struct Reverse;

impl Command for Reverse {
    fn name(&self) -> &str {
        "reverse"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("reverse").category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Reverses the table."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert, inverse, flip"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            example: "[0,1,2,3] | reverse",
            description: "Reverse the items",
            result: Some(Value::List {
                vals: vec![
                    Value::test_int(3),
                    Value::test_int(2),
                    Value::test_int(1),
                    Value::test_int(0),
                ],
                span: Span::test_data(),
            }),
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.metadata();

        #[allow(clippy::needless_collect)]
        let v: Vec<_> = input.into_iter().collect();
        let iter = v.into_iter().rev();
        Ok(iter
            .into_pipeline_data(engine_state.ctrlc.clone())
            .set_metadata(metadata))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Reverse {})
    }
}
