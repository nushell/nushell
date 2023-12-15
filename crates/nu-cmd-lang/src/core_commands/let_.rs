use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type};

#[derive(Clone)]
pub struct Let;

impl Command for Let {
    fn name(&self) -> &str {
        "let"
    }

    fn usage(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("let")
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

    fn extra_usage(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn is_parser_keyword(&self) -> bool {
        true
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["set", "const"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let var_id = call
            .positional_nth(0)
            .expect("checked through parser")
            .as_var()
            .expect("internal error: missing variable");

        let block_id = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing right hand side");

        let block = engine_state.get_block(block_id);
        let pipeline_data = eval_block(engine_state, stack, block, input, true, false)?;
        stack.add_var(var_id, pipeline_data.into_value(call.head));
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Set a variable to a value",
                example: "let x = 10",
                result: None,
            },
            Example {
                description: "Set a variable to the result of an expression",
                example: "let x = 10 + 100",
                result: None,
            },
            Example {
                description: "Set a variable based on the condition",
                example: "let x = if false { -1 } else { 1 }",
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

        test_examples(Let {})
    }

    #[test]
    fn test_command_type() {
        assert!(matches!(Let.command_type(), CommandType::Keyword));
    }
}
