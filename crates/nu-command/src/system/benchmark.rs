use std::time::Instant;

use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, Signature, SyntaxShape, Value};

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
        Signature::build("benchmark").required(
            "block",
            SyntaxShape::Block(Some(vec![])),
            "the block to run",
        )
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let block = engine_state.get_block(block);

        let mut stack = stack.collect_captures(&block.captures);
        let start_time = Instant::now();
        eval_block(engine_state, &mut stack, block, PipelineData::new())?.into_value();

        let end_time = Instant::now();

        let output = Value::Duration {
            val: (end_time - start_time).as_nanos() as i64,
            span: call.head,
        };

        Ok(output.into_pipeline_data())
    }
}
