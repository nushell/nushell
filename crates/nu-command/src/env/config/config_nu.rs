use nu_engine::command_prelude::*;
use nu_protocol::PipelineMetadata;

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

    fn examples(&self) -> Vec<Example> {
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
        if default_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_default_config(), head)
                .into_pipeline_data_with_metadata(
                    PipelineMetadata::default()
                        .with_content_type("application/x-nuscript".to_string().into()),
                ));
        }

        // `--doc` flag handling
        if doc_flag {
            let head = call.head;
            return Ok(Value::string(nu_utils::get_doc_config(), head)
                .into_pipeline_data_with_metadata(
                    PipelineMetadata::default()
                        .with_content_type("application/x-nuscript".to_string().into()),
                ));
        }

        super::config_::start_editor("config-path", engine_state, stack, call)
    }
}
