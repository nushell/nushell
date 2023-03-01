use nu_engine::{eval_block, CallExt};
use nu_protocol::{
    ast::Call,
    engine::{Closure, Command, EngineState, Stack},
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type,
    Value,
};
use std::time::Instant;

#[derive(Clone)]
pub struct TimeIt;

impl Command for TimeIt {
    fn name(&self) -> &str {
        "timeit"
    }

    fn usage(&self) -> &str {
        "Time the running time of a closure."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("timeit")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run",
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
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_block(capture_block.block_id);

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let mut stack = stack.captures_to_stack(&capture_block.captures);

        // In order to provide the pipeline as a positional, it must be converted into a value.
        // But because pipelines do not have Clone, this one has to be cloned as a value
        // and then converted back into a pipeline for eval_block().
        // So, the metadata must be saved here and restored at that point.
        let input_metadata = input.metadata();
        let input_val = input.into_value(call.head);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, input_val.clone());
            }
        }

        // Get the start time after all other computation has been done.
        let start_time = Instant::now();
        eval_block(
            engine_state,
            &mut stack,
            block,
            input_val.into_pipeline_data_with_metadata(input_metadata),
            redirect_stdout,
            redirect_stderr,
        )?
        .into_value(call.head);

        let end_time = Instant::now();

        let output = Value::Duration {
            val: (end_time - start_time).as_nanos() as i64,
            span: call.head,
        };

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
        ]
    }
}

#[test]
// Due to difficulty in observing side-effects from time closures,
// checks that the closures have run correctly must use the filesystem.
fn test_time_closure() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("test_time_closure", |dirs, _| {
        let inp = [
            r#"[2 3 4] | timeit { to nuon | save foo.txt }"#,
            "open foo.txt",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "[2, 3, 4]");
    });
}

#[test]
fn test_time_closure_2() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("test_time_closure", |dirs, _| {
        let inp = [
            r#"[2 3 4] | timeit {|e| {result: $e} | to nuon | save foo.txt }"#,
            "open foo.txt",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "{result: [2, 3, 4]}");
    });
}
