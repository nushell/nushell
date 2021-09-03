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
        Signature::build("each")
            .required(
                "var_name",
                SyntaxShape::Variable,
                "name of the looping variable",
            )
            .required("block", SyntaxShape::Block, "the block to run")
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let var_id = call.positional[0]
            .as_var()
            .expect("internal error: missing variable");

        let block = call.positional[1]
            .as_block()
            .expect("internal error: expected block");
        let context = context.clone();

        match input {
            Value::List { val, .. } => Ok(Value::List {
                val: val
                    .map(move |x| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block);

                        let state = context.enter_scope();
                        state.add_var(var_id, x.clone());

                        //FIXME: DON'T UNWRAP
                        eval_block(&state, block, Value::nothing()).unwrap()
                    })
                    .into_value_stream(),
                span: call.head,
            }),
            _ => Ok(Value::nothing()),
        }
    }
}
