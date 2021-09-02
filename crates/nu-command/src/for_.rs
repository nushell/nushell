use nu_engine::{eval_block, eval_expression};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{Signature, Span, SyntaxShape, Value};

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
                SyntaxShape::Keyword(b"in".to_vec(), Box::new(SyntaxShape::Int)),
                "range of the loop",
            )
            .required("block", SyntaxShape::Block, "the block to run")
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
        let end_val = eval_expression(context, keyword_expr)?;

        let block = call.positional[2]
            .as_block()
            .expect("internal error: expected block");
        let engine_state = context.engine_state.borrow();
        let block = engine_state.get_block(block);

        let state = context.enter_scope();

        let mut x = Value::Int {
            val: 0,
            span: Span::unknown(),
        };

        loop {
            if x == end_val {
                break;
            } else {
                state.add_var(var_id, x.clone());
                eval_block(&state, block)?;
            }
            if let Value::Int { ref mut val, .. } = x {
                *val += 1
            }
        }
        Ok(Value::Nothing {
            span: call.positional[0].span,
        })
    }
}
