use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
};

#[derive(Clone)]
pub struct All;

impl Command for All {
    fn name(&self) -> &str {
        "all"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name())
            .input_output_types(vec![
                (Type::List(Box::new(Type::Any)), Type::Bool),
                (Type::Table(vec![]), Type::Bool),
            ])
            .required(
                "predicate",
                SyntaxShape::RowCondition,
                "the expression, or block, that must evaluate to a boolean",
            )
            .category(Category::Filters)
    }

    fn usage(&self) -> &str {
        "Test if every element of the input fulfills a predicate expression."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["every", "and"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Check if each row's status is the string 'UP'",
                example: "[[status]; [UP] [UP]] | all status == UP",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description:
                    "Check that all of the values are even, using the built-in $it variable",
                example: "[2 4 6 8] | all ($it mod 2) == 0",
                result: Some(Value::test_bool(true)),
            },
            Example {
                description: "Check that all of the values are even, using a block",
                example: "[2 4 6 8] | all {|e| $e mod 2 == 0 }",
                result: Some(Value::test_bool(true)),
            },
        ]
    }
    // This is almost entirely a copy-paste of `any`'s run(), so make sure any changes to this are
    // reflected in the other!! (Or, you could figure out a way for both of them to use
    // the same function...)
    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block_id = capture_block.block_id;

        let block = engine_state.get_block(block_id);
        let var_id = block.signature.get_positional(0).and_then(|arg| arg.var_id);
        let mut stack = stack.captures_to_stack(&capture_block.captures);

        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        for value in input.into_interruptible_iter(ctrlc) {
            // with_env() is used here to ensure that each iteration uses
            // a different set of environment variables.
            // Hence, a 'cd' in the first loop won't affect the next loop.
            stack.with_env(&orig_env_vars, &orig_env_hidden);

            if let Some(var_id) = var_id {
                stack.add_var(var_id, value.clone());
            }

            let eval = eval_block(
                &engine_state,
                &mut stack,
                block,
                value.into_pipeline_data(),
                call.redirect_stdout,
                call.redirect_stderr,
            );
            match eval {
                Err(e) => {
                    return Err(e);
                }
                Ok(pipeline_data) => {
                    if !pipeline_data.into_value(span).is_true() {
                        return Ok(Value::Bool { val: false, span }.into_pipeline_data());
                    }
                }
            }
        }
        Ok(Value::Bool { val: true, span }.into_pipeline_data())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(All)
    }
}
