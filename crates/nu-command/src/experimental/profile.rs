use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Signature, SyntaxShape, Type,
};

#[derive(Clone)]
pub struct Profile;

impl Command for Profile {
    fn name(&self) -> &str {
        "profile"
    }

    fn usage(&self) -> &str {
        "Time the running time of a closure"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("profile")
            .required(
                "closure",
                SyntaxShape::Closure(Some(vec![SyntaxShape::Any])),
                "the closure to run",
            )
            .named(
                "max-depth",
                SyntaxShape::Int,
                "How many levels of blocks to step into (default: 1)",
                Some('d'),
            )
            .input_output_types(vec![
                (Type::Any, Type::Duration),
                (Type::Nothing, Type::Duration),
            ])
            .allow_variants_without_examples(true)
            .category(Category::System)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
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
        stack.debug_depth =
            if let Some(depth) = call.get_flag::<i64>(engine_state, &mut stack, "max-depth")? {
                depth
            } else {
                1
            };

        let output = eval_block(
            engine_state,
            &mut stack,
            block,
            input_val.into_pipeline_data_with_metadata(input_metadata),
            redirect_stdout,
            redirect_stderr,
        )?
        .into_value(call.head);

        Ok(output.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![]
    }
}

#[test]
// Due to difficulty in observing side-effects from benchmark closures,
// checks that the closures have run correctly must use the filesystem.
fn test_benchmark_closure() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("test_benchmark_closure", |dirs, _| {
        let inp = [
            r#"[2 3 4] | profile { to nuon | save foo.txt }"#,
            "open foo.txt",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "[2, 3, 4]");
    });
}

#[test]
fn test_benchmark_closure_2() {
    use nu_test_support::{nu, nu_repl_code, playground::Playground};
    Playground::setup("test_benchmark_closure", |dirs, _| {
        let inp = [
            r#"[2 3 4] | profile {|e| {result: $e} | to nuon | save foo.txt }"#,
            "open foo.txt",
        ];
        let actual_repl = nu!(cwd: dirs.test(), nu_repl_code(&inp));
        assert_eq!(actual_repl.err, "");
        assert_eq!(actual_repl.out, "{result: [2, 3, 4]}");
    });
}
