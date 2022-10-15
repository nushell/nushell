use nu_engine::env_to_strings;
use nu_protocol::{
    ast::Call,
    engine::{Command, EngineState, Stack},
    Category, Example, PipelineData, ShellError, Signature, Span, Spanned,
};

use crate::ExternalCommand;

use super::utils::get_editor;

#[derive(Clone)]
pub struct ConfigEnv;

impl Command for ConfigEnv {
    fn name(&self) -> &str {
        "config env"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Env)
    }

    fn usage(&self) -> &str {
        "Edit nu environment configurations"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "allow user to open and update nu env",
            example: "config env",
            result: None,
        }]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let env_vars_str = env_to_strings(engine_state, stack)?;
        let mut config_path = match nu_path::config_dir() {
            Some(path) => path,
            None => {
                return Err(ShellError::GenericError(
                    "Could not find nu env path".to_string(),
                    "Could not find nu env path".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };
        config_path.push("nushell");
        let mut nu_config = config_path.clone();
        nu_config.push("env.nu");

        let name = Spanned {
            item: get_editor(engine_state, stack)?,
            span: call.head,
        };

        let args = vec![Spanned {
            item: nu_config.to_string_lossy().to_string(),
            span: Span { start: 0, end: 0 },
        }];

        let command = ExternalCommand {
            name,
            args,
            arg_keep_raw: vec![false],
            redirect_stdout: false,
            redirect_stderr: false,
            env_vars: env_vars_str,
        };

        command.run_with_input(engine_state, stack, input, true)
    }
}
