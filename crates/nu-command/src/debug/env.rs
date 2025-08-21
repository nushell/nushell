use nu_engine::{command_prelude::*, env_to_strings};

#[derive(Clone)]
pub struct DebugEnv;

impl Command for DebugEnv {
    fn name(&self) -> &str {
        "debug env"
    }

    fn signature(&self) -> Signature {
        Signature::new(self.name())
            .input_output_type(Type::Nothing, Type::record())
            .category(Category::Debug)
    }

    fn description(&self) -> &str {
        "Show environment variables as external commands would get it."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(PipelineData::value(
            env_to_strings(engine_state, stack)?.into_value(call.head),
            None,
        ))
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Get PATH variable that externals see",
                example: "debug env | get PATH!",
                result: None,
            },
            Example {
                description: "Create a .env file",
                example: r#"debug env | transpose key value | each {$"($in.key)=($in.value | to json)"} | save .env"#,
                result: None,
            },
        ]
    }
}
