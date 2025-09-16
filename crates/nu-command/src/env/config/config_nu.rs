use nu_engine::command_prelude::*;
use nu_utils::ConfigFileKind;

#[derive(Clone)]
pub struct ConfigNu;

impl Command for ConfigNu {
    fn name(&self) -> &str {
        "config nu"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::Any)])
            .switch(
                "default",
                "Print the internal default `config.nu` file instead.",
                Some('d'),
            )
            .switch(
                "doc",
                "Print a commented `config.nu` with documentation instead.",
                Some('s'),
            )
    }

    fn description(&self) -> &str {
        "Edit nu configurations."
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "open user's config.nu in the default editor",
                example: "config nu",
                result: None,
            },
            Example {
                description: "pretty-print a commented `config.nu` that explains common settings",
                example: "config nu --doc | nu-highlight",
                result: None,
            },
            Example {
                description: "pretty-print the internal `config.nu` file which is loaded before user's config",
                example: "config nu --default | nu-highlight",
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
        super::config_::handle_call(ConfigFileKind::Config, engine_state, stack, call)
    }
}
