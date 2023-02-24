use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct Const;

impl Command for Const {
    fn name(&self) -> &str {
        "const"
    }

    fn usage(&self) -> &str {
        "Create a parse-time constant."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("const")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("const_name", SyntaxShape::VarWithOptType, "constant name")
            .required(
                "initial_value",
                SyntaxShape::Keyword(b"=".to_vec(), Box::new(SyntaxShape::Expression)),
                "equals sign followed by constant value",
            )
            .category(Category::Core)
    }

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "let"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let var_id = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_var()
            .expect("internal error: missing variable");

        if let Some(constval) = engine_state.find_constant(var_id, &[]) {
            // Instead of creating a second copy of the value in the stack, we could change
            // stack.get_var() to check engine_state.find_constant().
            stack.add_var(var_id, constval.clone());

            Ok(PipelineData::empty())
        } else {
            Err(ShellError::NushellFailedSpanned(
                "Missing Constant".to_string(),
                "constant not added by the parser".to_string(),
                call.head,
            ))
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a new parse-time constant.",
                example: "const x = 10",
                result: None,
            },
            Example {
                description: "Create a composite constant value",
                example: "const x = { a: 10, b: 20 }",
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

        test_examples(Const {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Const.command_type(), CommandType::Keyword));
    }
}
