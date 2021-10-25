use std::time::Instant;

use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{PipelineData, Signature, SyntaxShape, Value};

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
        context: &EvaluationContext,
        call: &Call,
        _input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let block = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let block = context.engine_state.get_block(block);

        let state = context.enter_scope();
        let start_time = Instant::now();
        eval_block(&state, block, PipelineData::new())?;
        let end_time = Instant::now();
        println!("{} ms", (end_time - start_time).as_millis());
        Ok(PipelineData::new())
    }
}
