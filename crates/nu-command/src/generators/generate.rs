use nu_engine::{command_prelude::*, ClosureEval};
use nu_protocol::engine::Closure;

#[derive(Clone)]
pub struct Generate;

impl Command for Generate {
    fn name(&self) -> &str {
        "generate"
    }

    fn signature(&self) -> Signature {
        Signature::build("generate")
            .input_output_types(vec![(Type::Nothing, Type::List(Box::new(Type::Any)))])
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "Generator function.",
            )
            .optional("initial", SyntaxShape::Any, "Initial value.")
            .allow_variants_without_examples(true)
            .category(Category::Generators)
    }

    fn usage(&self) -> &str {
        "Generate a list of values by successively invoking a closure."
    }

    fn extra_usage(&self) -> &str {
        r#"The generator closure accepts a single argument and returns a record
containing two optional keys: 'out' and 'next'. Each invocation, the 'out'
value, if present, is added to the stream. If a 'next' key is present, it is
used as the next argument to the closure, otherwise generation stops.
"#
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["unfold", "stream", "yield", "expand"]
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "generate {|i| if $i <= 10 { {out: $i, next: ($i + 2)} }} 0",
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
                example:
                    "generate {|fib| {out: $fib.0, next: [$fib.1, ($fib.0 + $fib.1)]} } [0, 1]",
                description: "Generate a continuous stream of Fibonacci numbers",
                result: None,
            },
            Example {
                example:
                    "generate {|fib=[0, 1]| {out: $fib.0, next: [$fib.1, ($fib.0 + $fib.1)]} }",
                description:
                    "Generate a continuous stream of Fibonacci numbers, using default parameters",
                result: None,
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
        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let initial: Option<Value> = call.opt(engine_state, stack, 1)?;
        let block = engine_state.get_block(closure.block_id);
        let mut closure = ClosureEval::new(engine_state, stack, closure);

        // A type of Option<S> is used to represent state. Invocation
        // will stop on None. Using Option<S> allows functions to output
        // one final value before stopping.
        let mut state = Some(get_initial_state(initial, &block.signature, call.head)?);
        let iter = std::iter::from_fn(move || {
            let arg = state.take()?;

            let (output, next_input) = match closure.run_with_value(arg) {
                // no data -> output nothing and stop.
                Ok(PipelineData::Empty) => (None, None),

                Ok(PipelineData::Value(value, ..)) => {
                    let span = value.span();
                    match value {
                        // {out: ..., next: ...} -> output and continue
                        Value::Record { val, .. } => {
                            let iter = val.into_owned().into_iter();
                            let mut out = None;
                            let mut next = None;
                            let mut err = None;

                            for (k, v) in iter {
                                if k.eq_ignore_ascii_case("out") {
                                    out = Some(v);
                                } else if k.eq_ignore_ascii_case("next") {
                                    next = Some(v);
                                } else {
                                    let error = ShellError::GenericError {
                                        error: "Invalid block return".into(),
                                        msg: format!("Unexpected record key '{}'", k),
                                        span: Some(span),
                                        help: None,
                                        inner: vec![],
                                    };
                                    err = Some(Value::error(error, head));
                                    break;
                                }
                            }

                            if err.is_some() {
                                (err, None)
                            } else {
                                (out, next)
                            }
                        }

                        // some other value -> error and stop
                        _ => {
                            let error = ShellError::GenericError {
                                error: "Invalid block return".into(),
                                msg: format!("Expected record, found {}", value.get_type()),
                                span: Some(span),
                                help: None,
                                inner: vec![],
                            };

                            (Some(Value::error(error, head)), None)
                        }
                    }
                }

                Ok(other) => {
                    let error = other
                        .into_value(head)
                        .map(|val| ShellError::GenericError {
                            error: "Invalid block return".into(),
                            msg: format!("Expected record, found {}", val.get_type()),
                            span: Some(val.span()),
                            help: None,
                            inner: vec![],
                        })
                        .unwrap_or_else(|err| err);

                    (Some(Value::error(error, head)), None)
                }

                // error -> error and stop
                Err(error) => (Some(Value::error(error, head)), None),
            };

            // We use `state` to control when to stop, not `output`. By wrapping
            // it in a `Some`, we allow the generator to output `None` as a valid output
            // value.
            state = next_input;
            Some(output)
        });

        Ok(iter
            .flatten()
            .into_pipeline_data(call.head, engine_state.signals().clone()))
    }
}

fn get_initial_state(
    initial: Option<Value>,
    signature: &Signature,
    span: Span,
) -> Result<Value, ShellError> {
    match initial {
        Some(v) => Ok(v),
        None => {
            // the initial state should be referred from signature
            if !signature.optional_positional.is_empty()
                && signature.optional_positional[0].default_value.is_some()
            {
                Ok(signature.optional_positional[0]
                    .default_value
                    .clone()
                    .expect("Already checked default value"))
            } else {
                Err(ShellError::GenericError {
                    error: "The initial value is missing".to_string(),
                    msg: "Missing initial value".to_string(),
                    span: Some(span),
                    help: Some(
                        "Provide an <initial> value as an argument to generate, or assign a default value to the closure parameter"
                            .to_string(),
                    ),
                    inner: vec![],
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Generate {})
    }
}
