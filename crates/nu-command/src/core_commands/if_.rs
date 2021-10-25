use nu_engine::{eval_block, eval_expression};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct If;

impl Command for If {
    fn name(&self) -> &str {
        "if"
    }

    fn usage(&self) -> &str {
        "Conditionally run a block."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("if")
            .required("cond", SyntaxShape::Expression, "condition")
            .required("then_block", SyntaxShape::Block(Some(vec![])), "then block")
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
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = &call.positional[0];
        let then_block = call.positional[1]
            .as_block()
            .expect("internal error: expected block");
        let else_case = call.positional.get(2);

        let result = eval_expression(context, cond)?;
        match result {
            Value::Bool { val, span } => {
                if val {
                    let block = context.engine_state.get_block(then_block);
                    let state = context.enter_scope();
                    eval_block(&state, block, input)
                } else if let Some(else_case) = else_case {
                    if let Some(else_expr) = else_case.as_keyword() {
                        if let Some(block_id) = else_expr.as_block() {
                            let block = context.engine_state.get_block(block_id);
                            let state = context.enter_scope();
                            eval_block(&state, block, input)
                        } else {
                            eval_expression(context, else_expr).map(|x| x.into_pipeline_data())
                        }
                    } else {
                        eval_expression(context, else_case).map(|x| x.into_pipeline_data())
                    }
                } else {
                    Ok(PipelineData::new())
                }
            }
            _ => Err(ShellError::CantConvert("bool".into(), result.span()?)),
        }
    }
}
