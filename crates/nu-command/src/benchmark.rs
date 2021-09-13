use std::time::Instant;

use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

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
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let block = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let engine_state = context.engine_state.borrow();
        let block = engine_state.get_block(block);

        let state = context.enter_scope();
        let start_time = Instant::now();
        eval_block(&state, block, Value::nothing())?;
        let end_time = Instant::now();
        println!("{} ms", (end_time - start_time).as_millis());
        Ok(Value::Nothing {
            span: call.positional[0].span,
        })
    }
}
