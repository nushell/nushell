use nu_engine::{eval_block, eval_expression};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoValueStream, Signature, SyntaxShape, Value};

pub struct For;

impl Command for For {
    fn name(&self) -> &str {
        "for"
    }

    fn usage(&self) -> &str {
        "Loop over a range"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("for")
            .required(
                "var_name",
                SyntaxShape::Variable,
                "name of the looping variable",
            )
            .required(
                "range",
                SyntaxShape::Keyword(
                    b"in".to_vec(),
                    Box::new(SyntaxShape::List(Box::new(SyntaxShape::Int))),
                ),
                "range of the loop",
            )
            .required(
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
        let var_id = call.positional[0]
            .as_var()
            .expect("internal error: missing variable");

        let keyword_expr = call.positional[1]
            .as_keyword()
            .expect("internal error: missing keyword");
        let values = eval_expression(context, keyword_expr)?;

        let block = call.positional[2]
            .as_block()
            .expect("internal error: expected block");
        let context = context.clone();

        match values {
            Value::Stream { stream, .. } => Ok(Value::Stream {
                stream: stream
                    .map(move |x| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block);

                        let state = context.enter_scope();
                        state.add_var(var_id, x);

                        //FIXME: DON'T UNWRAP
                        eval_block(&state, block, Value::nothing()).unwrap()
                    })
                    .into_value_stream(),
                span: call.head,
            }),
            Value::List { vals: val, .. } => Ok(Value::List {
                vals: val
                    .into_iter()
                    .map(move |x| {
                        let engine_state = context.engine_state.borrow();
                        let block = engine_state.get_block(block);

                        let state = context.enter_scope();
                        state.add_var(var_id, x);

                        //FIXME: DON'T UNWRAP
                        eval_block(&state, block, Value::nothing()).unwrap()
                    })
                    .collect(),
                span: call.head,
            }),
            _ => Ok(Value::nothing()),
        }
    }
}
