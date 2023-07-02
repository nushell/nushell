use nu_engine::{eval_block, eval_expression_with_input, CallExt};
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Matcher, Stack};
use nu_protocol::{
    Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Match;

impl Command for Match {
    fn name(&self) -> &str {
        "match"
    }

    fn usage(&self) -> &str {
        "Conditionally run a block on a matched value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("match")
            .input_output_types(vec![(Type::Any, Type::Any)])
            .required("value", SyntaxShape::Any, "value to check")
            .required(
                "match_block",
                SyntaxShape::MatchBlock,
                "block to run if check succeeds",
            )
            .category(Category::Core)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value: Value = call.req(engine_state, stack, 0)?;
        let block = call.positional_nth(1);

        if let Some(Expression {
            expr: Expr::MatchBlock(matches),
            ..
        }) = block
        {
            for match_ in matches {
                let mut match_variables = vec![];
                if match_.0.match_value(&value, &mut match_variables) {
                    // This case does match, go ahead and return the evaluated expression
                    for match_variable in match_variables {
                        stack.add_var(match_variable.0, match_variable.1);
                    }

                    if let Some(block_id) = match_.1.as_block() {
                        let block = engine_state.get_block(block_id);
                        return eval_block(
                            engine_state,
                            stack,
                            block,
                            input,
                            call.redirect_stdout,
                            call.redirect_stderr,
                        );
                    } else {
                        return eval_expression_with_input(
                            engine_state,
                            stack,
                            &match_.1,
                            input,
                            call.redirect_stdout,
                            call.redirect_stderr,
                        )
                        .map(|x| x.0);
                    }
                }
            }
        }

        Ok(PipelineData::Empty)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Match on a value in range",
                example: "match 3 { 1..10 => 'yes!' }",
                result: Some(Value::test_string("yes!")),
            },
            Example {
                description: "Match on a field in a record",
                example: "match {a: 100} { {a: $my_value} => { $my_value } }",
                result: Some(Value::test_int(100)),
            },
            Example {
                description: "Match with a catch-all",
                example: "match 3 { 1 => { 'yes!' }, _ => { 'no!' } }",
                result: Some(Value::test_string("no!")),
            },
            Example {
                description: "Match against a list",
                example: "match [1, 2, 3] { [$a, $b, $c] => { $a + $b + $c }, _ => 0 }",
                result: Some(Value::test_int(6)),
            },
            Example {
                description: "Match against pipeline input",
                example: "{a: {b: 3}} | match $pipe {{a: { $b }} => ($b + 10) }",
                result: Some(Value::test_int(13)),
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

        test_examples(Match {})
    }
}
