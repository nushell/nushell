use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::engine::Closure;
use nu_utils::time::Instant;

#[derive(Clone)]
pub struct TimeIt;

impl Command for TimeIt {
    fn name(&self) -> &str {
        "timeit"
    }

    fn description(&self) -> &str {
        "Time how long it takes a closure to run."
    }

    fn extra_description(&self) -> &str {
        "Any pipeline input given to this command is passed to the closure. Note that streaming inputs may affect timing results, and it is recommended to add a `collect` command before this if the input is a stream.

This command will bubble up any errors encountered when running the closure. The return pipeline of the closure is collected into a value and then discarded if `--output` is not set."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("timeit")
            .required("command", SyntaxShape::Closure(None), "The closure to run.")
            .switch("output", "Include the closure output.", Some('o'))
            .input_output_types(vec![
                (Type::Any, Type::Duration),
                (Type::Nothing, Type::Duration),
                (
                    Type::Any,
                    Type::Record(
                        vec![
                            ("time".into(), Type::Duration),
                            ("output".into(), Type::Any),
                        ]
                        .into(),
                    ),
                ),
                (
                    Type::Nothing,
                    Type::Record(
                        vec![
                            ("time".into(), Type::Duration),
                            ("output".into(), Type::Any),
                        ]
                        .into(),
                    ),
                ),
            ])
            .allow_variants_without_examples(true)
            .category(Category::Debug)
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["timing", "timer", "benchmark", "measure"]
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        // reset outdest, so the command can write to stdout and stderr.
        let stack = &mut stack.push_redirection(None, None);

        let include_output = call.has_flag(engine_state, stack, "output")?;
        let closure: Closure = call.req(engine_state, stack, 0)?;
        let closure = ClosureEvalOnce::new_preserve_out_dest(engine_state, stack, closure);

        // Get the start time after all other computation has been done.
        let start_time = Instant::now();
        let closure_output = closure.run_with_input(input)?.into_value(call.head)?;
        let time = Value::duration(start_time.elapsed().as_nanos() as i64, call.head);

        let output = if include_output {
            Value::record(
                record! {
                "time" => time,
                "output" => closure_output
                },
                call.head,
            )
        } else {
            time
        };

        Ok(output.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            #[cfg(not(test))]
            Example {
                description: "Time a closure containing one command.",
                example: "timeit { sleep 500ms }",
                result: Some(Value::test_duration(500_631_800)),
            },
            Example {
                description: "Time a closure with an input value.",
                example: "'A really long string' | timeit { split chars }",
                result: None,
            },
            Example {
                description: "Time a closure with an input stream.",
                example: "open some_file.txt | collect | timeit { split chars }",
                result: None,
            },
            Example {
                description: "Time a closure containing a pipeline.",
                example: "timeit { open some_file.txt | split chars }",
                result: None,
            },
            #[cfg(not(test))]
            Example {
                description: "Time a closure and also return the output.",
                example: "timeit --output { 'example text' }",
                result: Some(Value::test_record(record! {
                    "time" => Value::test_duration(14328),
                    "output" => Value::test_string("example text")
                })),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use nu_test_support::prelude::*;

    // Due to difficulty in observing side-effects from time closures,
    // checks that the closures have run correctly must use the filesystem.

    #[test]
    fn test_time_block() -> Result {
        Playground::setup("test_time_block", |dirs, _| {
            let _: Value = test()
                .cwd(dirs.test())
                .run("[2 3 4] | timeit {to nuon | save foo.txt }")?;
            let content = fs::read_to_string(dirs.test().join("foo.txt")).unwrap();
            assert_eq!(content, "[2, 3, 4]");
            Ok(())
        })
    }

    #[test]
    fn test_time_block_2() -> Result {
        Playground::setup("test_time_block", |dirs, _| {
            let _: Value = test()
                .cwd(dirs.test())
                .run("[2 3 4] | timeit {{result: $in} | to nuon | save foo.txt }")?;
            let content = fs::read_to_string(dirs.test().join("foo.txt")).unwrap();
            assert_eq!(content, "{result: [2, 3, 4]}");
            Ok(())
        })
    }
}
