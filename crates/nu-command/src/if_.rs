use nu_engine::{eval_block, eval_expression};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{ShellError, Signature, SyntaxShape, Value};

pub struct If;

impl Command for If {
    fn name(&self) -> &str {
        "if"
    }

    fn usage(&self) -> &str {
        "Create a variable and give it a value."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("if")
            .required("cond", SyntaxShape::Expression, "condition")
            .required("then_block", SyntaxShape::Block, "then block")
            .optional(
                "else",
                SyntaxShape::Keyword(b"else".to_vec(), Box::new(SyntaxShape::Expression)),
                "optional else followed by else block",
            )
    }

    fn run(
        &self,
        context: &EvaluationContext,
        call: &Call,
        _input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
        let cond = &call.positional[0];
        let then_block = call.positional[1]
            .as_block()
            .expect("internal error: expected block");
        let else_case = call.positional.get(2);

        let result = eval_expression(context, cond)?;
        match result {
            Value::Bool { val, span } => {
                let engine_state = context.engine_state.borrow();
                if val {
                    let block = engine_state.get_block(then_block);
                    let state = context.enter_scope();
                    eval_block(&state, block)
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let block = engine_state.get_block(block_id);
                            let state = context.enter_scope();
                            eval_block(&state, block)
                        } else {
                            eval_expression(context, else_expr)
                        }
                    } else {
                        eval_expression(context, else_case)
                    }
                } else {
                    Ok(Value::Nothing { span })
                }
            }
            _ => Err(ShellError::CantConvert("bool".into(), result.span())),
        }
    }
}
