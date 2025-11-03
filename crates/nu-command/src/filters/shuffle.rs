use nu_engine::command_prelude::*;
use rand::{prelude::SliceRandom, rng};

#[derive(Clone)]
pub struct Shuffle;

impl Command for Shuffle {
    fn name(&self) -> &str {
        "shuffle"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("shuffle")
            .input_output_types(vec![(
                Type::List(Box::new(Type::Any)),
                Type::List(Box::new(Type::Any)),
            )])
            .category(Category::Filters)
    }

    fn description(&self) -> &str {
        "Shuffle rows randomly."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let metadata = input.metadata();
        let mut values = input.into_iter_strict(call.head)?.collect::<Vec<_>>();
        values.shuffle(&mut rng());
        let iter = values.into_iter();
        Ok(iter.into_pipeline_data_with_metadata(
            call.head,
            engine_state.signals().clone(),
            metadata,
        ))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![Example {
            description: "Shuffle rows randomly (execute it several times and see the difference)",
            example: r#"[[version patch]; ['1.0.0' false] ['3.0.1' true] ['2.0.0' false]] | shuffle"#,
            result: None,
        }]
    }
}
