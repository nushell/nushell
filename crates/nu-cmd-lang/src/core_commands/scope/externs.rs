use nu_engine::{command_prelude::*, scope::ScopeData};

#[derive(Clone)]
pub struct ScopeExterns;

impl Command for ScopeExterns {
    fn name(&self) -> &str {
        "scope externs"
    }

    fn signature(&self) -> Signature {
        Signature::build("scope externs")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Output info on the known externals in the current scope."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let ctrlc = engine_state.ctrlc.clone();

        let mut scope_data = ScopeData::new(engine_state, stack);
        scope_data.populate_decls();

        Ok(scope_data.collect_externs(span).into_pipeline_data(ctrlc))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show the known externals in the current scope",
            example: "scope externs",
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

        test_examples(ScopeExterns {})
    }
}
