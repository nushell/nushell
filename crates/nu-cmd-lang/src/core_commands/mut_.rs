use nu_engine::command_prelude::*;
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct Mut;

impl Command for Mut {
    fn name(&self) -> &str {
        "mut"
    }

    fn description(&self) -> &str {
        "Create a mutable variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("mut")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("var_name", SyntaxShape::VarWithOptType, "Variable name.")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::MathExpression)),
                "Equals sign followed by value.",
            )
            .category(Category::Core)
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "mutable"]
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        _call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        eprintln!(
            "Tried to execute 'run' for the 'mut' command: this code path should never be reached in IR mode"
        );
        unreachable!()
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Set a mutable variable to a value, then update it",
                example: "mut x = 10; $x = 12",
                result: None,
            },
            Example {
                description: "Upsert a value inside a mutable data structure",
                example: "mut a = {b:{c:1}}; $a.b.c = 2",
                result: None,
            },
            Example {
                description: "Set a mutable variable to the result of an expression",
                example: "mut x = 10 + 100",
                result: None,
            },
            Example {
                description: "Set a mutable variable based on the condition",
                example: "mut x = if false { -1 } else { 1 }",
                result: None,
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use nu_protocol::engine::CommandType;

    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Mut {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Mut.command_type(), CommandType::Keyword));
    }
}
