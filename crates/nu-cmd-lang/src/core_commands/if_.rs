use nu_engine::command_prelude::*;
use nu_protocol::{
    engine::{CommandType, StateWorkingSet},
    eval_const::{eval_const_subexpression, eval_constant, eval_constant_with_input},
};

#[derive(Clone)]
pub struct If;

impl Command for If {
    fn name(&self) -> &str {
        "if"
    }

    fn description(&self) -> &str {
        "Conditionally run a block."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("if")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("cond", SyntaxShape::MathExpression, "Condition to check.")
            .required(
                "then_block",
                SyntaxShape::Block,
                "Block to run if check succeeds.",
            )
            .optional(
                "else_expression",
                SyntaxShape::Keyword(
                    b"else".to_vec(),
                    Box::new(SyntaxShape::OneOf(vec![
                        SyntaxShape::Block,
                        SyntaxShape::Expression,
                    ])),
                ),
                "Expression or block to run when the condition is false.",
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

    fn is_const(&self) -> bool {
        true
    }

    fn run_const(
        &self,
        working_set: &StateWorkingSet,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call = call.assert_ast_call()?;
        let cond = call.positional_nth(0).expect("checked through parser");
        let then_expr = call.positional_nth(1).expect("checked through parser");
        let then_block = then_expr
            .as_block()
            .ok_or_else(|| ShellError::TypeMismatch {
                err_message: "expected block".into(),
                span: then_expr.span,
            })?;
        let else_case = call.positional_nth(2);

        if eval_constant(working_set, cond)?.as_bool()? {
            let block = working_set.get_block(then_block);
            eval_const_subexpression(working_set, block, input, block.span.unwrap_or(call.head))
        } else if let Some(else_case) = else_case {
            if let Some(else_expr) = else_case.as_keyword() {
                if let Some(block_id) = else_expr.as_block() {
                    let block = working_set.get_block(block_id);
                    eval_const_subexpression(
                        working_set,
                        block,
                        input,
                        block.span.unwrap_or(call.head),
                    )
                } else {
                    eval_constant_with_input(working_set, else_expr, input)
                }
            } else {
                eval_constant_with_input(working_set, else_case, input)
            }
        } else {
            Ok(PipelineData::empty())
        }
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
            "Tried to execute 'run' for the 'if' command: this code path should never be reached in IR mode"
        );
        unreachable!()
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["else", "conditional"]
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Output a value if a condition matches, otherwise return nothing",
                example: "if 2 < 3 { 'yes!' }",
                result: Some(Value::test_string("yes!")),
            },
            Example {
                description: "Output a value if a condition matches, else return another value",
                example: "if 5 < 3 { 'yes!' } else { 'no!' }",
                result: Some(Value::test_string("no!")),
            },
            Example {
                description: "Chain multiple if's together",
                example: "if 5 < 3 { 'yes!' } else if 4 < 5 { 'no!' } else { 'okay!' }",
                result: Some(Value::test_string("no!")),
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

        test_examples(If {})
    }
}
