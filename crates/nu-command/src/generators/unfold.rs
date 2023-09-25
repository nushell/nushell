use itertools::unfold;

use nu_engine::{eval_block_with_early_return, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Span, Spanned, SyntaxShape, Type, Value,
};

#[derive(Clone)]
pub struct Unfold;

impl Command for Unfold {
    fn name(&self) -> &str {
        "unfold"
    }

    fn signature(&self) -> Signature {
        Signature::build("unfold")
            .input_output_types(vec![
                (Type::Nothing, Type::List(Box::new(Type::Any))),
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
            ])
            .required("initial", SyntaxShape::Any, "initial value")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "generator function",
            )
            .allow_variants_without_examples(true)
            .category(Category::Generators)
    }

    fn usage(&self) -> &str {
        "Generate a list of values by successively invoking a closure."
    }

    fn extra_usage(&self) -> &str {
        r#"The generator closure accepts a single argument and returns a list containing
a value to output and the next argument to pass into the generator.

Generation stops when the closure does not return a "next" value.
Returning a list of more than two elements will result in an error."#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["generate", "stream"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "unfold 0 {|i| if $i <= 10 { [$i, ($i + 2)] }}",
                description: "Generate a sequence of numbers",
                result: Some(Value::list(
                    vec![
                        Value::test_int(0),
                        Value::test_int(2),
                        Value::test_int(4),
                        Value::test_int(6),
                        Value::test_int(8),
                        Value::test_int(10),
                    ],
                    Span::test_data(),
                )),
            },
            Example {
                example: "unfold [0, 1] {|fib| [$fib.0, [$fib.1, ($fib.0 + $fib.1)]] } | first 10",
                description: "Generate a stream of fibonacci numbers",
                result: Some(Value::list(
                    vec![
                        Value::test_int(0),
                        Value::test_int(1),
                        Value::test_int(1),
                        Value::test_int(2),
                        Value::test_int(3),
                        Value::test_int(5),
                        Value::test_int(8),
                        Value::test_int(13),
                        Value::test_int(21),
                        Value::test_int(34),
                    ],
                    Span::test_data(),
                )),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let initial: Value = call.req(engine_state, stack, 0)?;
        let capture_block: Spanned<Closure> = call.req(engine_state, stack, 1)?;
        let block_span = capture_block.span;
        let block = engine_state.get_block(capture_block.item.block_id).clone();
        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();
        let mut stack = stack.captures_to_stack(&capture_block.item.captures);
        let orig_env_vars = stack.env_vars.clone();
        let orig_env_hidden = stack.env_hidden.clone();
        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        // A type of Option<S> is used to represent state. Invocation
        // will stop on None. Using Option<S> allows functions to output
        // one final value before stopping.
        let iter = unfold(Some(initial), move |state| {
            let arg = match state {
                Some(state) => state.clone(),
                None => return None,
            };

            // with_env() is used here to ensure that each iteration uses
            // a different set of environment variables.
            // Hence, a 'cd' in the first loop won't affect the next loop.
            stack.with_env(&orig_env_vars, &orig_env_hidden);

            if let Some(var) = block.signature.get_positional(0) {
                if let Some(var_id) = &var.var_id {
                    stack.add_var(*var_id, arg.clone());
                }
            }

            let (output, next_input) = match eval_block_with_early_return(
                &engine_state,
                &mut stack,
                &block,
                arg.into_pipeline_data(),
                redirect_stdout,
                redirect_stderr,
            ) {
                // no data -> output nothing and stop.
                Ok(PipelineData::Empty) => (None, None),

                // []        -> output nothing and stop.
                // [a]       -> output `a` and stop.
                // [a b]     -> output `a` and continue with `b`.
                // [a b ...] -> error
                Ok(PipelineData::Value(Value::List { vals, .. }, ..)) => {
                    if vals.len() <= 2 {
                        let mut iter = vals.into_iter();
                        (iter.next(), iter.next())
                    } else {
                        let error = ShellError::GenericError(
                            "Invalid block return".to_string(),
                            "Generator returned a list with more than 2 elements".to_string(),
                            Some(block_span),
                            None,
                            Vec::new(),
                        );
                        (Some(Value::error(error, block_span)), None)
                    }
                }

                // single value -> output it and stop.
                Ok(v) => (Some(v.into_value(block_span)), None),

                // error -> output it and stop
                Err(error) => (Some(Value::error(error, block_span)), None),
            };

            *state = next_input;
            output
        });

        Ok(iter.into_pipeline_data(ctrlc))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Unfold {})
    }
}
