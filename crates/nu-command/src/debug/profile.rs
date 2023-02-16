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
        "Profile each pipeline element in a closure."
    }

    fn extra_usage(&self) -> &str {
        r#"The command collects run time of every pipeline element, recursively stepping into child closures
until a maximum depth. Optionally, it also collects the source code and intermediate values.

Current known limitations are:
* profiling data from subexpressions is not tracked
* it does not step into loop iterations"#
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
            .input_output_types(vec![(Type::Any, Type::Table(vec![]))])
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

        let profiling_metadata = Box::new(PipelineMetadata {
            data_source: DataSource::Profiling(vec![]),
        });

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
        .map(|m| *m)
        {
            Value::list(values, call.head)
        } else {
            Value::nothing(call.head)
        };

        Ok(result.into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description:
                "Profile some code, stepping into the `spam` command and collecting source.",
            example: r#"def spam [] { "spam" }; profile { spam | str length } -d 2 --source"#,
            result: None,
        }]
    }
}
