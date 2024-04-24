use nu_engine::{command_prelude::*, scope::ScopeData};

#[derive(Clone)]
pub struct ScopeAliases;

impl Command for ScopeAliases {
    fn name(&self) -> &str {
        "scope aliases"
    }

    fn signature(&self) -> Signature {
        Signature::build("scope aliases")
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .allow_variants_without_examples(true)
            .category(Category::Core)
    }

    fn usage(&self) -> &str {
        "Output info on the aliases in the current scope."
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

        Ok(scope_data.collect_aliases(span).into_pipeline_data(ctrlc))
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Show the aliases in the current scope",
            example: "scope aliases",
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

        test_examples(ScopeAliases {})
    }
}
