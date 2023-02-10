use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, ProfilingConfig, Stack};
use nu_protocol::{
    Category, DataSource, Example, IntoPipelineData, PipelineData, PipelineMetadata, Signature,
    Spanned, SyntaxShape, Type, Value,
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
            .switch("source", "Collect source code in the report", None)
            .switch("values", "Collect values in the report", None)
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
            .category(Category::Debug)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Spanned<Closure> = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_block(capture_block.item.block_id);

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let mut stack = stack.captures_to_stack(&capture_block.item.captures);

        let input_val = input.into_value(call.head);

        if let Some(var) = block.signature.get_positional(0) {
            if let Some(var_id) = &var.var_id {
                stack.add_var(*var_id, input_val.clone());
            }
        }

        stack.profiling_config = ProfilingConfig::new(
            call.get_flag::<i64>(engine_state, &mut stack, "max-depth")?
                .unwrap_or(1),
            call.has_flag("source"),
            call.has_flag("values"),
        );

        let profiling_metadata = PipelineMetadata {
            data_source: DataSource::Profiling(vec![]),
        };

        let result = if let Some(PipelineMetadata {
            data_source: DataSource::Profiling(values),
        }) = eval_block(
            engine_state,
            &mut stack,
            block,
            input_val.into_pipeline_data_with_metadata(profiling_metadata),
            redirect_stdout,
            redirect_stderr,
        )?
        .metadata()
        {
            Value::list(values, call.head)
        } else {
            Value::nothing(call.head)
        };

        Ok(result.into_pipeline_data())
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
