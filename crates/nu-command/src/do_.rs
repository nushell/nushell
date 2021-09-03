use nu_engine::{eval_block, eval_expression};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, SyntaxShape, Value};

pub struct Do;

impl Command for Do {
    fn name(&self) -> &str {
        "do"
    }

    fn usage(&self) -> &str {
        "Run a block"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("do").required("block", SyntaxShape::Block, "the block to run")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let block = &call.positional[0];

        let out = eval_expression(context, &block)?;

        match out {
            Value::Block { val: block_id, .. } => {
                let engine_state = context.engine_state.borrow();
                let block = engine_state.get_block(block_id);
                eval_block(context, block, input)
            }
            _ => Ok(Value::nothing()),
        }
    }
}
