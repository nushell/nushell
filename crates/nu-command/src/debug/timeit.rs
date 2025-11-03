use nu_engine::{ClosureEvalOnce, command_prelude::*};
use nu_protocol::engine::Closure;
use web_time::Instant;

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

This command will bubble up any errors encountered when running the closure. The return pipeline of the closure is collected into a value and then discarded."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("timeit")
            .required("command", SyntaxShape::Closure(None), "The closure to run.")
            .input_output_types(vec![
                (Type::Any, Type::Duration),
                (Type::Nothing, Type::Duration),
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

        let closure: Closure = call.req(engine_state, stack, 0)?;
        let closure = ClosureEvalOnce::new_preserve_out_dest(engine_state, stack, closure);

        // Get the start time after all other computation has been done.
        let start_time = Instant::now();
        closure.run_with_input(input)?.into_value(call.head)?;
        let time = start_time.elapsed();

        let output = Value::duration(time.as_nanos() as i64, call.head);
        Ok(output.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Time a closure containing one command",
                example: "timeit { sleep 500ms }",
                result: None,
            },
            Example {
                description: "Time a closure with an input value",
                example: "'A really long string' | timeit { split chars }",
                result: None,
            },
            Example {
                description: "Time a closure with an input stream",
                example: "open some_file.txt | collect | timeit { split chars }",
                result: None,
            },
            Example {
                description: "Time a closure containing a pipeline",
                example: "timeit { open some_file.txt | split chars }",
                result: None,
            },
        ]
    }
}

#[test]
// Due to difficulty in observing side-effects from time closures,
// checks that the closures have run correctly must use the filesystem.
fn test_time_block() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("test_time_block", |dirs, _| {
        let inp = [
            r#"[2 3 4] | timeit {to nuon | save foo.txt }"#,
            "open foo.txt",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "[2, 3, 4]");
    });
}

#[test]
fn test_time_block_2() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("test_time_block", |dirs, _| {
        let inp = [
            r#"[2 3 4] | timeit {{result: $in} | to nuon | save foo.txt }"#,
            "open foo.txt",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "{result: [2, 3, 4]}");
    });
}
