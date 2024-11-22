use nu_engine::command_prelude::*;

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
                "sample",
                "Print a commented, sample `config.nu` file instead.",
                Some('s'),
            )
        // TODO: Signature narrower than what run actually supports theoretically
    }

    fn description(&self) -> &str {
        "Edit nu configurations."
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "open user's config.nu in the default editor",
                example: "config nu",
                result: None,
            },
            Example {
                description: "pretty-print a commented, sample `config.nu` that explains common settings",
                example: "config nu --sample | nu-highlight",
                result: None,
            },
            Example {
                description:
                    "pretty-print the internal `config.nu` file which is loaded before user's config",
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
        let default_flag = call.has_flag(engine_state, stack, "default")?;
        let sample_flag = call.has_flag(engine_state, stack, "sample")?;
        if default_flag && sample_flag {
            return Err(ShellError::IncompatibleParameters {
                left_message: "can't use `--default` at the same time".into(),
                left_span: call.get_flag_span(stack, "default").expect("has flag"),
                right_message: "because of `--sample`".into(),
                right_span: call.get_flag_span(stack, "sample").expect("has flag"),
            });
        }

        // `--default` flag handling
        if default_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_config(), head).into_pipeline_data());
        }

        // `--sample` flag handling
        if sample_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_sample_config(), head).into_pipeline_data());
        }

        super::config_::start_editor("config-path", engine_state, stack, call)
    }
}
