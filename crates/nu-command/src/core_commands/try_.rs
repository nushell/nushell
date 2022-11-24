use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Block, Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, Signature, SyntaxShape, Type, Value};

#[derive(Clone)]
pub struct Try;

impl Command for Try {
    fn name(&self) -> &str {
        "try"
    }

    fn usage(&self) -> &str {
        "Try to run a block, if it fails optionally run a catch block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("try")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("try_block", SyntaxShape::Block, "block to run")
            .optional(
                "else_expression",
                SyntaxShape::Keyword(b"catch".to_vec(), Box::new(SyntaxShape::Block)),
                "block to run if try block fails",
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
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let try_block: Block = call.req(engine_state, stack, 0)?;
        let catch_block: Option<Block> = call.opt(engine_state, stack, 1)?;

        let try_block = engine_state.get_block(try_block.block_id);

        let result = eval_block(engine_state, stack, try_block, input, false, false);

        match result {
            Err(_) | Ok(PipelineData::Value(Value::Error { .. }, ..)) => {
                if let Some(catch_block) = catch_block {
                    let catch_block = engine_state.get_block(catch_block.block_id);
                    eval_block(
                        engine_state,
                        stack,
                        catch_block,
                        PipelineData::new(call.head),
                        false,
                        false,
                    )
                } else {
                    Ok(PipelineData::new(call.head))
                }
            }
            Ok(output) => Ok(output),
        }
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Try to run a missing command",
                example: "try { asdfasdf }",
                result: None,
            },
            Example {
                description: "Try to run a missing command",
                example: "try { asdfasdf } catch { echo 'missing' } ",
                result: Some(Value::test_string("missing")),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Try {})
    }
}
