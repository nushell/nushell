use nu_engine::command_prelude::*;
use nu_utils::ConfigFileKind;

#[derive(Clone)]
pub struct ConfigEnv;

impl Command for ConfigEnv {
    fn name(&self) -> &str {
        "config env"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch(
                "default",
                "Print the internal default `env.nu` file instead.",
                Some('d'),
            )
            .switch(
                "doc",
                "Print a commented `env.nu` with documentation instead.",
                Some('s'),
            )
        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn description(&self) -> &str {
        "Edit nu environment configurations."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "open user's env.nu in the default editor",
                example: "config env",
                result: None,
            },
            Example {
                description: "pretty-print a commented `env.nu` that explains common settings",
                example: "config env --doc | nu-highlight,",
                result: None,
            },
            Example {
                description: "pretty-print the internal `env.nu` file which is loaded before the user's environment",
                example: "config env --default | nu-highlight,",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        super::config_::handle_call(ConfigFileKind::Env, engine_state, stack, call)
    }
}
