use nu_engine::{command_prelude::*, scope::ScopeData};

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
            .category(Category::Core)
    }

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example<'_>> {
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
