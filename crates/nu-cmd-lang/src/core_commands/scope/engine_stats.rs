use nu_engine::scope::ScopeData;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Type};

#[derive(Clone)]
pub struct ScopeEngineStats;

impl Command for ScopeEngineStats {
    fn name(&self) -> &str {
        "scope engine-stats"
    }

    fn signature(&self) -> Signature {
        Signature::build("scope engine-stats")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Output stats on the engine in the current state."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let scope_data = ScopeData::new(engine_state, stack);

        Ok(scope_data.collect_engine_state(span).into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show the stats on the current engine state",
            example: "scope engine-stats",
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(ScopeEngineStats {})
    }
}
