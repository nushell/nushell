use nu_engine::{command_prelude::*, get_full_help};

mod landlock;

pub use landlock::Landlock;

#[derive(Clone)]
pub struct Self_;

impl Command for Self_ {
    fn name(&self) -> &str {
        "self"
    }

    fn signature(&self) -> Signature {
        Signature::build("self")
            .category(Category::Experimental)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Commands for changing the status of the main Nushell process."
    }

    fn extra_description(&self) -> &str {
        "You must use one of the following subcommands. Using this command as-is will only produce this help message."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::string(get_full_help(self, engine_state, stack), call.head).into_pipeline_data())
    }
}
