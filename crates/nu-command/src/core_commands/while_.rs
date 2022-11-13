use nu_engine::{eval_block, eval_expression, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct While;

impl Command for While {
    fn name(&self) -> &str {
        "while"
    }

    fn usage(&self) -> &str {
        "Conditionally run a block in a loop."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("while")
            .required("cond", SyntaxShape::Expression, "condition to check")
            .required(
                "block",
                SyntaxShape::Block,
                "block to loop if check succeeds",
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

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = call.positional_nth(0).expect("checked through parser");
        let block: Block = call.req(engine_state, stack, 1)?;

        loop {
            let result = eval_expression(engine_state, stack, cond)?;
            match &result {
                Value::Bool { val, .. } => {
                    if *val {
                        let block = engine_state.get_block(block.block_id);
                        eval_block(
                            engine_state,
                            stack,
                            block,
                            PipelineData::new(call.head),
                            call.redirect_stdout,
                            call.redirect_stderr,
                        )?
                        .into_value(call.head);
                    } else {
                        break;
                    }
                }
                x => {
                    return Err(ShellError::CantConvert(
                        "bool".into(),
                        x.get_type().to_string(),
                        result.span()?,
                        None,
                    ))
                }
            }
        }
        Ok(PipelineData::new(call.head))
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
