use nu_engine::{command_prelude::*, get_eval_block};
use nu_protocol::engine::CommandType;

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
        vec!["set", "const"]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        let call = call.assert_ast_call()?;
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
        let eval_block = get_eval_block(engine_state);
        let stack = &mut stack.start_collect_value();
        let pipeline_data = eval_block(engine_state, stack, block, input)?;
        let value = pipeline_data.into_value(call.head)?;

        // if given variable type is Glob, and our result is string
        // then nushell need to convert from Value::String to Value::Glob
        // it's assigned by demand, then it's not quoted, and it's required to expand
        // if we pass it to other commands.
        let var_type = &engine_state.get_var(var_id).ty;
        let val_span = value.span();
        let value = match value {
            Value::String { val, .. } if var_type == &Type::Glob => {
                Value::glob(val, false, val_span)
            }
            value => value,
        };

        stack.add_var(var_id, value);
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
