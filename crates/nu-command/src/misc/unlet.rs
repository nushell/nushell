use nu_engine::command_prelude::*;

#[derive(Clone)]
pub struct DeleteVar;

impl Command for DeleteVar {
    fn name(&self) -> &str {
        "unlet"
    }

    fn description(&self) -> &str {
        "Delete variables from nushell memory, making them unrecoverable."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("unlet")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .rest(
                "rest",
                SyntaxShape::Any,
                "The variables to delete (pass as $variable_name).",
            )
            .category(Category::Experimental)
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // Compiled specially by the IR compiler (`compile_unlet`). This path is never used
        // when running in IR mode.
        eprintln!(
            "Tried to execute 'run' for the 'unlet' command: this code path should never be reached in IR mode"
        );
        unreachable!()
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "let x = 42; unlet $x",
                description: "Delete a variable from memory.",
                result: None,
            },
            Example {
                example: "let x = 1; let y = 2; unlet $x $y",
                description: "Delete multiple variables from memory.",
                result: None,
            },
            Example {
                example: "unlet $nu",
                description: "Attempting to delete a built-in variable fails.",
                result: None,
            },
            Example {
                example: "unlet 42",
                description: "Attempting to delete a non-variable fails.",
                result: None,
            },
        ]
    }
}
