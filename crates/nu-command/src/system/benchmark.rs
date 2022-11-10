use std::time::Instant;

use nu_engine::{eval_block, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Closure, Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Benchmark;

impl Command for Benchmark {
    fn name(&self) -> &str {
        "benchmark"
    }

    fn usage(&self) -> &str {
        "Time the running time of a block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("benchmark")
            .required("block", SyntaxShape::Block, "the block to run")
            .category(Category::System)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let capture_block: Closure = call.req(engine_state, stack, 0)?;
        let block = engine_state.get_block(capture_block.block_id);

        let redirect_stdout = call.redirect_stdout;
        let redirect_stderr = call.redirect_stderr;

        let mut stack = stack.captures_to_stack(&capture_block.captures);
        let start_time = Instant::now();
        eval_block(
            engine_state,
            &mut stack,
            block,
            PipelineData::new(call.head),
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
        vec![Example {
            description: "Benchmarks a command within a block",
            example: "benchmark { sleep 500ms }",
            result: None,
        }]
    }
}
