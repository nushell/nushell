use super::utils::chain_error_with_input;
use nu_engine::{ClosureEval, ClosureEvalOnce, command_prelude::*};
use nu_protocol::{Signals, engine::Closure, shell_error::generic::GenericError};
use rayon::prelude::*;
use std::{
    sync::mpsc::{self, RecvTimeoutError},
    time::Duration,
};

const STREAM_BUFFER_SIZE: usize = 64;
const CTRL_C_CHECK_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone)]
pub struct ParEach;

impl Command for ParEach {
    fn name(&self) -> &str {
        "par-each"
    }

    fn description(&self) -> &str {
        "Run a closure on each row of the input list in parallel, creating a new list with the results."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("par-each")
            .input_output_types(vec![
                (
                    Type::List(Box::new(Type::Any)),
                    Type::List(Box::new(Type::Any)),
                ),
                (Type::table(), Type::List(Box::new(Type::Any))),
                (Type::Any, Type::Any),
            ])
            .named(
                "threads",
                SyntaxShape::Int,
                "The number of threads to use.",
                Some('t'),
            )
            .switch(
                "keep-order",
                "Keep sequence of output same as the order of input.",
                Some('k'),
            )
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "The closure to run.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                example: "[1 2 3] | par-each {|e| $e * 2 }",
                description: "Multiplies each number. Note that the list will become arbitrarily disordered.",
                result: None,
            },
            Example {
                example: "[1 2 3] | par-each --keep-order {|e| $e * 2 }",
                description: "Multiplies each number, keeping an original order.",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(4),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: "1..3 | enumerate | par-each {|p| update item ($p.item * 2)} | sort-by item | get item",
                description: "Enumerate and sort-by can be used to reconstruct the original order.",
                result: Some(Value::test_list(vec![
                    Value::test_int(2),
                    Value::test_int(4),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: "[foo bar baz] | par-each {|e| $e + '!' } | sort",
                description: "Output can still be sorted afterward.",
                result: Some(Value::test_list(vec![
                    Value::test_string("bar!"),
                    Value::test_string("baz!"),
                    Value::test_string("foo!"),
                ])),
            },
            Example {
                example: r#"[1 2 3] | enumerate | par-each { |e| if $e.item == 2 { $"found 2 at ($e.index)!"} }"#,
                description: "Iterate over each element, producing a list showing indexes of any 2s.",
                result: Some(Value::test_list(vec![Value::test_string("found 2 at 1!")])),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        fn create_pool(num_threads: usize, head: Span) -> Result<rayon::ThreadPool, ShellError> {
            match rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
            {
                Err(e) => Err(e).map_err(|e| {
                    ShellError::Generic(GenericError::new(
                        "Error creating thread pool",
                        e.to_string(),
                        head,
                    ))
                }),
                Ok(pool) => Ok(pool),
            }
        }

        let head = call.head;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let threads: Option<usize> = call.get_flag(engine_state, stack, "threads")?;
        let max_threads = threads.unwrap_or(0);
        let keep_order = call.has_flag(engine_state, stack, "keep-order")?;
        let signals = engine_state.signals().clone();

        let mut input = input.into_stream_or_original(engine_state);
        let metadata = input.take_metadata();

        // A helper function sorts the output if needed
        let apply_order = |mut vec: Vec<(usize, Value)>| {
            if keep_order {
                // It runs inside the rayon's thread pool so parallel sorting can be used.
                // There are no identical indexes, so unstable sorting can be used.
                vec.par_sort_unstable_by_key(|(index, _)| *index);
            }

            vec.into_iter().map(|(_, val)| val)
        };

        match input {
            PipelineData::Empty => Ok(PipelineData::empty()),
            PipelineData::Value(value, ..) => {
                let span = value.span();
                match value {
                    Value::List { vals, .. } => {
                        let pool = create_pool(max_threads, head)?;
                        if keep_order {
                            Ok(pool.install(|| {
                                let par_iter = vals.into_par_iter().enumerate();
                                let mapped =
                                    parallel_closure_map(engine_state, stack, &closure, par_iter);
                                apply_order(mapped.collect())
                                    .into_pipeline_data(span, signals.clone())
                            }))
                        } else {
                            let par_iter = vals.into_par_iter();
                            Ok(stream_parallel_values(
                                engine_state,
                                stack,
                                closure.clone(),
                                pool,
                                span,
                                signals.clone(),
                                par_iter,
                            ))
                        }
                    }
                    Value::Range { val, .. } => {
                        let pool = create_pool(max_threads, head)?;
                        if keep_order {
                            Ok(pool.install(|| {
                                let par_iter = val
                                    .into_range_iter(span, signals.clone())
                                    .enumerate()
                                    .par_bridge();
                                let mapped =
                                    parallel_closure_map(engine_state, stack, &closure, par_iter);
                                apply_order(mapped.collect())
                                    .into_pipeline_data(span, signals.clone())
                            }))
                        } else {
                            let par_iter = val.into_range_iter(span, signals.clone()).par_bridge();
                            Ok(stream_parallel_values(
                                engine_state,
                                stack,
                                closure.clone(),
                                pool,
                                span,
                                signals.clone(),
                                par_iter,
                            ))
                        }
                    }
                    // This match allows non-iterables to be accepted,
                    // which is currently considered undesirable (Nov 2022).
                    value => {
                        ClosureEvalOnce::new(engine_state, stack, closure).run_with_value(value)
                    }
                }
            }
            PipelineData::ListStream(stream, ..) => {
                let pool = create_pool(max_threads, head)?;
                if keep_order {
                    Ok(pool.install(|| {
                        let par_iter = stream.into_iter().enumerate().par_bridge();
                        let mapped = parallel_closure_map(engine_state, stack, &closure, par_iter);

                        apply_order(mapped.collect()).into_pipeline_data(head, signals.clone())
                    }))
                } else {
                    let par_iter = stream.into_iter().par_bridge();
                    Ok(stream_parallel_values(
                        engine_state,
                        stack,
                        closure.clone(),
                        pool,
                        head,
                        signals.clone(),
                        par_iter,
                    ))
                }
            }
            PipelineData::ByteStream(stream, ..) => {
                if let Some(chunks) = stream.chunks() {
                    let pool = create_pool(max_threads, head)?;
                    if keep_order {
                        Ok(pool.install(|| {
                            let par_iter = chunks
                                .enumerate()
                                .map(move |(idx, val)| {
                                    (idx, val.unwrap_or_else(|err| Value::error(err, head)))
                                })
                                .par_bridge();
                            let mapped =
                                parallel_closure_map(engine_state, stack, &closure, par_iter);
                            apply_order(mapped.collect()).into_pipeline_data(head, signals.clone())
                        }))
                    } else {
                        let par_iter = chunks
                            .map(move |val| val.unwrap_or_else(|err| Value::error(err, head)))
                            .par_bridge();
                        Ok(stream_parallel_values(
                            engine_state,
                            stack,
                            closure.clone(),
                            pool,
                            head,
                            signals.clone(),
                            par_iter,
                        ))
                    }
                } else {
                    Ok(PipelineData::empty())
                }
            }
        }
        .and_then(|x| x.filter(|v| !v.is_nothing(), engine_state.signals()))
        .map(|data| data.set_metadata(metadata))
    }
}

fn stream_parallel_values(
    engine_state: &EngineState,
    stack: &Stack,
    closure: Closure,
    pool: rayon::ThreadPool,
    span: Span,
    signals: Signals,
    input: impl ParallelIterator<Item = Value> + 'static,
) -> PipelineData {
    let (tx, rx) = mpsc::sync_channel(STREAM_BUFFER_SIZE);
    let worker_engine_state = engine_state.clone();
    let worker_stack = stack.clone();
    let worker_signals = signals.clone();

    pool.install(|| {
        rayon::spawn(move || {
            let map_signals = worker_signals.clone();
            let send_signals = worker_signals.clone();

            let _ = input
                .map_init(
                    move || ClosureEval::new(&worker_engine_state, &worker_stack, closure.clone()),
                    move |closure_eval, value| {
                        if map_signals.interrupted() {
                            return Err(());
                        }

                        let value = run_closure_on_value(closure_eval, value);

                        if map_signals.interrupted() {
                            Err(())
                        } else {
                            Ok(value)
                        }
                    },
                )
                .try_for_each(move |value| match value {
                    Ok(value) => {
                        if send_signals.interrupted() {
                            Err(())
                        } else {
                            tx.send(value).map_err(|_| ())
                        }
                    }
                    Err(()) => Err(()),
                });
        });
    });

    ReceiverIter::new(rx, signals).into_pipeline_data(span, Signals::empty())
}

// Polls channel reads so Ctrl+C can stop blocked receives promptly.
struct ReceiverIter {
    receiver: mpsc::Receiver<Value>,
    signals: Signals,
}

impl ReceiverIter {
    fn new(receiver: mpsc::Receiver<Value>, signals: Signals) -> Self {
        Self { receiver, signals }
    }
}

impl Iterator for ReceiverIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.signals.interrupted() {
                return None;
            }

            match self.receiver.recv_timeout(CTRL_C_CHECK_INTERVAL) {
                Ok(value) => return Some(value),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
}

fn run_closure_on_value(closure_eval: &mut ClosureEval, value: Value) -> Value {
    let span = value.span();
    let is_error = value.is_error();

    closure_eval
        .run_with_value(value)
        .and_then(|data| data.into_value(span))
        .unwrap_or_else(|err| Value::error(chain_error_with_input(err, is_error, span), span))
}

fn parallel_closure_map(
    engine_state: &EngineState,
    stack: &mut Stack,
    closure: &Closure,
    input: impl ParallelIterator<Item = (usize, Value)>,
) -> impl ParallelIterator<Item = (usize, Value)> {
    input.map_init(
        move || ClosureEval::new(engine_state, stack, closure.clone()),
        |closure_eval, (index, value)| {
            let value = run_closure_on_value(closure_eval, value);

            (index, value)
        },
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ParEach)
    }
}
