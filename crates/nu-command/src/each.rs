use nu_engine::eval_block;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoValueStream, Signature, SyntaxShape, Value};

pub struct Each;

impl Command for Each {
    fn name(&self) -> &str {
        "each"
    }

    fn usage(&self) -> &str {
        "Run a block on each element of input"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("each").required("block", SyntaxShape::Block, "the block to run")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let block_id = call.positional[0]
            .as_block()
            .expect("internal error: expected block");
        let context = context.clone();

        match input {
            Value::List { val, .. } => Ok(Value::List {
                val: val
                    .into_iter()
                    .map(move |x| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block_id);

                        let state = context.enter_scope();
                        if let Some(var) = block.signature.required_positional.first() {
                            if let Some(var_id) = &var.var_id {
                                state.add_var(*var_id, x);
                            }
                        }

                        match eval_block(&state, block, Value::nothing()) {
                            Ok(v) => v,
                            Err(err) => Value::Error { err },
                        }
                    })
                    .collect(),
                span: call.head,
            }),
            Value::ValueStream { stream, .. } => Ok(Value::ValueStream {
                stream: stream
                    .map(move |x| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block_id);

                        let state = context.enter_scope();
                        if let Some(var) = block.signature.required_positional.first() {
                            if let Some(var_id) = &var.var_id {
                                state.add_var(*var_id, x);
                            }
                        }

                        match eval_block(&state, block, Value::nothing()) {
                            Ok(v) => v,
                            Err(err) => Value::Error { err },
                        }
                    })
                    .into_value_stream(),
                span: call.head,
            }),
            _ => Ok(Value::nothing()),
        }
    }
}
