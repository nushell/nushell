use nu_engine::{
    command_prelude::*, get_eval_block, get_eval_expression, get_eval_expression_with_input,
};
use nu_protocol::engine::{CommandType, Matcher};

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
            .required("value", SyntaxShape::Any, "Value to check.")
            .required(
                "match_block",
                SyntaxShape::MatchBlock,
                "Block to run if check succeeds.",
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

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let value: Value = call.req(engine_state, stack, 0)?;
        let matches = call
            .positional_nth(1)
            .expect("checked through parser")
            .as_match_block()
            .expect("missing match block");

        let eval_expression = get_eval_expression(engine_state);
        let eval_expression_with_input = get_eval_expression_with_input(engine_state);
        let eval_block = get_eval_block(engine_state);

        let mut match_variables = vec![];
        for (pattern, expr) in matches {
            if pattern.match_value(&value, &mut match_variables) {
                // This case does match, go ahead and return the evaluated expression
                for (id, value) in match_variables.drain(..) {
                    stack.add_var(id, value);
                }

                let guard_matches = if let Some(guard) = &pattern.guard {
                    let Value::Bool { val, .. } = eval_expression(engine_state, stack, guard)?
                    else {
                        return Err(ShellError::MatchGuardNotBool { span: guard.span });
                    };

                    val
                } else {
                    true
                };

                if guard_matches {
                    return if let Some(block_id) = expr.as_block() {
                        let block = engine_state.get_block(block_id);
                        eval_block(engine_state, stack, block, input)
                    } else {
                        eval_expression_with_input(engine_state, stack, expr, input).map(|x| x.0)
                    };
                }
            } else {
                match_variables.clear();
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
                example: "{a: {b: 3}} | match $in {{a: { $b }} => ($b + 10) }",
                result: Some(Value::test_int(13)),
            },
            Example {
                description: "Match with a guard",
                example: "match [1 2 3] {
        [$x, ..$y] if $x == 1 => { 'good list' },
        _ => { 'not a very good list' }
    }
    ",
                result: Some(Value::test_string("good list")),
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
