use nu_engine::eval_expression;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Where;

impl Command for Where {
    fn name(&self) -> &str {
        "where"
    }

    fn usage(&self) -> &str {
        "Filter values based on a condition."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("where").required("cond", SyntaxShape::RowCondition, "condition")
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = call.positional[0].clone();

        let engine_state = engine_state.clone();
        let mut stack = stack.enter_scope();

        let (var_id, cond) = match cond {
            Expression {
                expr: Expr::RowCondition(var_id, expr),
                ..
            } => (var_id, expr),
            _ => return Err(ShellError::InternalError("Expected row condition".into())),
        };

        match input {
            PipelineData::Stream(stream) => Ok(stream
                .filter(move |value| {
                    stack.add_var(var_id, value.clone());

                    let result = eval_expression(&engine_state, &mut stack, &cond);

                    match result {
                        Ok(result) => result.is_true(),
                        _ => false,
                    }
                })
                .into_pipeline_data()),
            PipelineData::Value(Value::List { vals, .. }) => Ok(vals
                .into_iter()
                .filter(move |value| {
                    stack.add_var(var_id, value.clone());

                    let result = eval_expression(&engine_state, &mut stack, &cond);

                    match result {
                        Ok(result) => result.is_true(),
                        _ => false,
                    }
                })
                .into_pipeline_data()),
            PipelineData::Value(x) => {
                stack.add_var(var_id, x.clone());

                let result = eval_expression(&engine_state, &mut stack, &cond)?;

                if result.is_true() {
                    Ok(x.into_pipeline_data())
                } else {
                    Ok(PipelineData::new())
                }
            }
        }
    }
}
