use nu_engine::command_prelude::*;
use nu_protocol::{engine::CommandType, shell_error::generic::GenericError};

#[derive(Clone)]
pub struct Let;

impl Command for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn description(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("let")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .allow_variants_without_examples(true)
            .required(
                "var_name",
                SyntaxShape::VarWithOptType,
                "The variable name to create.",
            )
            .optional(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "Equals sign followed by value.",
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        "This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "const"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // The IR compiler turns a direct `let` call into instructions (`compile_let`). This only
        // runs for an indirect call, like `%($name)` or a plugin's `call_decl`, which passes
        // values rather than a variable to bind.
        Err(GenericError::new(
            "Can't call `let` indirectly",
            "`let` creates the variable written in its call, like `let x = 1`",
            call.head,
        )
        .into())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Set a variable to a value (no output).",
                example: "let x = 10",
                result: None,
            },
            Example {
                description: "Set a variable to the result of an expression (no output).",
                example: "let x = 10 + 100",
                result: None,
            },
            Example {
                description: "Set a variable based on the condition (no output).",
                example: "let x = if false { -1 } else { 1 }",
                result: None,
            },
            Example {
                description: "Set a variable to the output of a pipeline.",
                example: "ls | let files",
                result: None,
            },
            Example {
                description: "Use let in the middle of a pipeline to assign and pass the value.",
                example: "10 | let x | $x + 5",
                result: Some(Value::test_int(15)),
            },
            Example {
                description: "Use let in the middle of a pipeline, then consume value with $in.",
                example: "10 | let x | $in + 5",
                result: Some(Value::test_int(15)),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(Let)
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Let.command_type(), CommandType::Keyword));
    }
}
