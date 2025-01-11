use nu_engine::command_prelude::*;

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

    fn examples(&self) -> Vec<Example> {
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
        let default_flag = call.has_flag(engine_state, stack, "default")?;
        let doc_flag = call.has_flag(engine_state, stack, "doc")?;
        if default_flag && doc_flag {
            return Err(ShellError::IncompatibleParameters {
                left_message: "can't use `--default` at the same time".into(),
                left_span: call.get_flag_span(stack, "default").expect("has flag"),
                right_message: "because of `--doc`".into(),
                right_span: call.get_flag_span(stack, "doc").expect("has flag"),
            });
        }
        // `--default` flag handling
        if call.has_flag(engine_state, stack, "default")? {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_env(), head).into_pipeline_data());
        }

        // `--doc` flag handling
        if doc_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_doc_env(), head).into_pipeline_data());
        }

        super::config_::start_editor("env-path", engine_state, stack, call)
    }
}
