use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::{engine::Closure, shell_error::io::IoError};
use std::{sync::mpsc, thread};

#[derive(Clone)]
pub struct Interleave;

impl Command for Interleave {
    fn name(&self) -> &str {
        "interleave"
    }

    fn description(&self) -> &str {
        "Read multiple streams in parallel and combine them into one stream."
    }

    fn extra_description(&self) -> &str {
        r#"This combinator is useful for reading output from multiple commands.

If input is provided to `interleave`, the input will be combined with the
output of the closures. This enables `interleave` to be used at any position
within a pipeline.

Because items from each stream will be inserted into the final stream as soon
as they are available, there is no guarantee of how the final output will be
ordered. However, the order of items from any given stream is guaranteed to be
preserved as they were in that stream.

If interleaving streams in a fair (round-robin) manner is desired, consider
using `zip { ... } | flatten` instead."#
    }

    fn signature(&self) -> Signature {
        Signature::build("interleave")
            .input_output_types(vec![
                (Type::List(Type::Any.into()), Type::List(Type::Any.into())),
                (Type::Nothing, Type::List(Type::Any.into())),
            ])
            .named(
                "buffer-size",
                SyntaxShape::Int,
                "Number of items to buffer from the streams. Increases memory usage, but can help \
                    performance when lots of output is produced.",
                Some('b'),
            )
            .rest(
                "closures",
                SyntaxShape::Closure(None),
                "The closures that will generate streams to be combined.",
            )
            .allow_variants_without_examples(true)
            .category(Category::Filters)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                example: "seq 1 50 | wrap a | interleave { seq 1 50 | wrap b }",
                description: r#"Read two sequences of numbers into separate columns of a table.
Note that the order of rows with 'a' columns and rows with 'b' columns is arbitrary."#,
                result: None,
            },
            Example {
                example: "seq 1 3 | interleave { seq 4 6 } | sort",
                description: "Read two sequences of numbers, one from input. Sort for consistency.",
                result: Some(Value::test_list(vec![
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(3),
                    Value::test_int(4),
                    Value::test_int(5),
                    Value::test_int(6),
                ])),
            },
            Example {
                example: r#"interleave { "foo\nbar\n" | lines } { "baz\nquux\n" | lines } | sort"#,
                description: "Read two sequences, but without any input. Sort for consistency.",
                result: Some(Value::test_list(vec![
                    Value::test_string("bar"),
                    Value::test_string("baz"),
                    Value::test_string("foo"),
                    Value::test_string("quux"),
                ])),
            },
            Example {
                example: r#"(
interleave
    { nu -c "print hello; print world" | lines | each { "greeter: " ++ $in } }
    { nu -c "print nushell; print rocks" | lines | each { "evangelist: " ++ $in } }
)"#,
                description: "Run two commands in parallel and annotate their output.",
                result: None,
            },
            Example {
                example: "seq 1 20000 | interleave --buffer-size 16 { seq 1 20000 } | math sum",
                description: "Use a buffer to increase the performance of high-volume streams.",
                result: None,
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
        let head = call.head;
        let closures: Vec<Closure> = call.rest(engine_state, stack, 0)?;
        let buffer_size: usize = call
            .get_flag(engine_state, stack, "buffer-size")?
            .unwrap_or(0);

        let (tx, rx) = mpsc::sync_channel(buffer_size);

        // Spawn the threads for the input and closure outputs
        (!input.is_nothing())
            .then(|| Ok(input))
            .into_iter()
            .chain(closures.into_iter().map(|closure| {
                ClosureEvalOnce::new(engine_state, stack, closure)
                    .run_with_input(PipelineData::empty())
            }))
            .try_for_each(|stream| {
                stream.and_then(|stream| {
                    // Then take the stream and spawn a thread to send it to our channel
                    let tx = tx.clone();
                    thread::Builder::new()
                        .name("interleave consumer".into())
                        .spawn(move || {
                            for value in stream {
                                if tx.send(value).is_err() {
                                    // Stop sending if the channel is dropped
                                    break;
                                }
                            }
                        })
                        .map(|_| ())
                        .map_err(|err| IoError::new(err, head, None).into())
                })
            })?;

        // Now that threads are writing to the channel, we just return it as a stream
        Ok(rx
            .into_iter()
            .into_pipeline_data(head, engine_state.signals().clone()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() {
        use crate::test_examples;

        test_examples(Interleave {})
    }
}
