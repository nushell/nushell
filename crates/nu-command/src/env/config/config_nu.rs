use nu_engine::env_to_strings;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Type,
};

use super::utils::{gen_command, get_editor};

#[derive(Clone)]
pub struct ConfigNu;

impl Command for ConfigNu {
    fn name(&self) -> &str {
        "config nu"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .category(Category::Env)
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn usage(&self) -> &str {
        "Edit nu configurations"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "allow user to open and update nu config",
            example: "config nu",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let env_vars_str = env_to_strings(engine_state, stack)?;
        let mut config_path = match nu_path::config_dir() {
            Some(path) => path,
            None => {
                return Err(ShellError::GenericError(
                    "Could not find nu config path".to_string(),
                    "Could not find nu config path".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };
        config_path.push("nushell");
        let mut nu_config = config_path.clone();
        nu_config.push("config.nu");

        let (item, config_args) = get_editor(engine_state, stack)?;

        gen_command(call.head, nu_config, item, config_args, env_vars_str).run_with_input(
            engine_state,
            stack,
            input,
            true,
        )
    }
}
