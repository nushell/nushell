use nu_engine::{eval_block, eval_expression, eval_expression_with_input, CallExt};
use nu_protocol::ast::{Call, Expr, Expression, MatchPattern};
use nu_protocol::engine::{Block, Command, EngineState, Matcher, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
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
                    let output = eval_expression(engine_state, stack, &match_.1)?;

                    return Ok(output.into_pipeline_data());
                }
            }
        }

        Ok(PipelineData::Empty)

        // println!("Value: {:?}", value);
        // println!("Pattern: {:?}", pattern);

        // let mut variables = vec![];
        // let result = pattern.match_value(&value, &mut variables);

        // println!("Result: {}", result);
        // println!("Variables: {:?}", variables);

        // Ok(PipelineData::Empty)
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

        test_examples(Match {})
    }
}
