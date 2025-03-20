use nu_engine::{command_prelude::*, get_eval_block, get_eval_expression};
use nu_protocol::engine::CommandType;

#[derive(Clone)]
pub struct While;

impl Command for While {
    fn name(&self) -> &str {
        "while"
    }

    fn description(&self) -> &str {
        "Conditionally run a block in a loop."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("while")
            .input_output_types(vec![(Type::Nothing, Type::Nothing)])
            .allow_variants_without_examples(true)
            .required("cond", SyntaxShape::MathExpression, "Condition to check.")
            .required(
                "block",
                SyntaxShape::Block,
                "Block to loop if check succeeds.",
            )
            .category(Category::Core)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["loop"]
    }

    fn extra_description(&self) -> &str {
        r#"This command is a parser keyword. For details, check:
  https://www.nushell.sh/book/thinking_in_nu.html"#
    }

    fn command_type(&self) -> CommandType {
        CommandType::Keyword
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // This is compiled specially by the IR compiler. The code here is never used when
        // running in IR mode.
        let call = call.assert_ast_call()?;
        let head = call.head;
        let cond = call.positional_nth(0).expect("checked through parser");
        let block_id = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");

        let eval_expression = get_eval_expression(engine_state);
        let eval_block = get_eval_block(engine_state);

        let stack = &mut stack.push_redirection(None, None);

        loop {
            engine_state.signals().check(head)?;

            let result = eval_expression(engine_state, stack, cond)?;

            match &result {
                Value::Bool { val, .. } => {
                    if *val {
                        let block = engine_state.get_block(block_id);

                        match eval_block(engine_state, stack, block, PipelineData::empty()) {
                            Err(ShellError::Break { .. }) => break,
                            Err(ShellError::Continue { .. }) => continue,
                            Err(err) => return Err(err),
                            Ok(data) => data.drain()?,
                        }
                    } else {
                        break;
                    }
                }
                x => {
                    return Err(ShellError::CantConvert {
                        to_type: "bool".into(),
                        from_type: x.get_type().to_string(),
                        span: result.span(),
                        help: None,
                    })
                }
            }
        }
        Ok(PipelineData::empty())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Loop while a condition is true",
            example: "mut x = 0; while $x < 10 { $x = $x + 1 }",
            result: None,
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(While {})
    }
}
