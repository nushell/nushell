use nu_engine::{
    command_prelude::*, get_eval_block, get_eval_expression, get_eval_expression_with_input,
};
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

    fn usage(&self) -> &str {
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

    fn extra_usage(&self) -> &str {
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
        let cond = call.positional_nth(0).expect("checked through parser");
        let then_block = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");
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
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cond = call.positional_nth(0).expect("checked through parser");
        let then_block = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_block()
            .expect("internal error: missing block");
        let else_case = call.positional_nth(2);

        let eval_expression = get_eval_expression(engine_state);
        let eval_expression_with_input = get_eval_expression_with_input(engine_state);
        let eval_block = get_eval_block(engine_state);

        if eval_expression(engine_state, stack, cond)?.as_bool()? {
            let block = engine_state.get_block(then_block);
            eval_block(engine_state, stack, block, input)
        } else if let Some(else_case) = else_case {
            if let Some(else_expr) = else_case.as_keyword() {
                if let Some(block_id) = else_expr.as_block() {
                    let block = engine_state.get_block(block_id);
                    eval_block(engine_state, stack, block, input)
                } else {
                    eval_expression_with_input(engine_state, stack, else_expr, input)
                        .map(|res| res.0)
                }
            } else {
                eval_expression_with_input(engine_state, stack, else_case, input).map(|res| res.0)
            }
        } else {
            Ok(PipelineData::empty())
        }
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["else", "conditional"]
    }

    fn examples(&self) -> Vec<Example> {
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
