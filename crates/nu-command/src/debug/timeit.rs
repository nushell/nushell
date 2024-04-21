use nu_engine::{command_prelude::*, get_eval_block, get_eval_expression_with_input};
use std::time::Instant;

#[derive(Clone)]
pub struct TimeIt;

impl Command for TimeIt {
    fn name(&self) -> &str {
        "timeit"
    }

    fn usage(&self) -> &str {
        "Time the running time of a block."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("timeit")
            .required(
                "command",
                SyntaxShape::OneOf(vec![SyntaxShape::Block, SyntaxShape::Expression]),
                "The command or block to run.",
            )
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

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let command_to_run = call.positional_nth(0);

        // Get the start time after all other computation has been done.
        let start_time = Instant::now();

        // reset outdest, so the command can write to stdout and stderr.
        let stack = &mut stack.push_redirection(None, None);
        if let Some(command_to_run) = command_to_run {
            if let Some(block_id) = command_to_run.as_block() {
                let eval_block = get_eval_block(engine_state);
                let block = engine_state.get_block(block_id);
                eval_block(engine_state, stack, block, input)?
            } else {
                let eval_expression_with_input = get_eval_expression_with_input(engine_state);
                eval_expression_with_input(engine_state, stack, command_to_run, input)
                    .map(|res| res.0)?
            }
        } else {
            PipelineData::empty()
        }
        .into_value(call.head);

        let end_time = Instant::now();

        let output = Value::duration(
            end_time.saturating_duration_since(start_time).as_nanos() as i64,
            call.head,
        );

        Ok(output.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Times a command within a closure",
                example: "timeit { sleep 500ms }",
                result: None,
            },
            Example {
                description: "Times a command using an existing input",
                example: "http get https://www.nushell.sh/book/ | timeit { split chars }",
                result: None,
            },
            Example {
                description: "Times a command invocation",
                example: "timeit ls -la",
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
