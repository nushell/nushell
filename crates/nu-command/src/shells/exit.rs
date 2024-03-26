use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct Exit;

impl Command for Exit {
    fn name(&self) -> &str {
        "exit"
    }

    fn signature(&self) -> Signature {
        Signature::build("exit")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .optional(
                "exit_code",
                SyntaxShape::Int,
                "Exit code to return immediately with.",
            )
            .category(Category::Shells)
    }

    fn usage(&self) -> &str {
        "Exit Nu."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["quit", "close", "exit_code", "error_code", "logout"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let exit_code: Option<i64> = call.opt(engine_state, stack, 0)?;

        if let Some(exit_code) = exit_code {
            std::process::exit(exit_code as i32);
        }

        std::process::exit(0);
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Exit the current shell",
            example: "exit",
            result: None,
        }]
    }
}
