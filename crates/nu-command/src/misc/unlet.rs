use nu_engine::command_prelude::*;
use nu_protocol::shell_error::generic::GenericError;

#[derive(Clone)]
pub struct DeleteVar;

impl Command for DeleteVar {
    fn name(&self) -> &str {
        "unlet"
    }

    fn description(&self) -> &str {
        "Delete variables from nushell memory, making them unrecoverable."
    }

    fn extra_description(&self) -> &str {
        "A constant is hidden instead: it can't be used for the rest of the block, but code that \
         used it before the `unlet` keeps its value."
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
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // The IR compiler turns a direct `unlet` call into instructions (`compile_unlet`). This
        // only runs for an indirect call, like `%($name)` or a plugin's `call_decl`, which
        // passes values rather than variables.
        Err(GenericError::new(
            "Can't call `unlet` indirectly",
            "`unlet` deletes the variables written in its call, like `unlet $x`",
            call.head,
        )
        .into())
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
                example: "const x = 1; def f [] { $x }; unlet $x; f",
                description: "Hide a constant. A command defined before the `unlet` keeps its value.",
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
