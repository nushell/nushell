use super::utils::gen_command;
use nu_cmd_base::util::get_editor;
use nu_engine::{command_prelude::*, env_to_strings};

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
            .switch("default", "Print default `env.nu` file instead.", Some('d'))
        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn usage(&self) -> &str {
        "Edit nu environment configurations."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "allow user to open and update nu env",
                example: "config env",
                result: None,
            },
            Example {
                description: "allow user to print default `env.nu` file",
                example: "config env --default,",
                result: None,
            },
            Example {
                description: "allow saving the default `env.nu` locally",
                example: "config env --default | save -f ~/.config/nushell/default_env.nu",
                result: None,
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // `--default` flag handling
        if call.has_flag(engine_state, stack, "default")? {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_env(), head).into_pipeline_data());
        }

        let env_vars_str = env_to_strings(engine_state, stack)?;
        let nu_config = match engine_state.get_config_path("env-path") {
            Some(path) => path.clone(),
            None => {
                return Err(ShellError::GenericError {
                    error: "Could not find $nu.env-path".into(),
                    msg: "Could not find $nu.env-path".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
        };

        let (item, config_args) = get_editor(engine_state, stack, call.head)?;

        gen_command(call.head, nu_config, item, config_args, env_vars_str).run_with_input(
            engine_state,
            stack,
            input,
            true,
        )
    }
}
