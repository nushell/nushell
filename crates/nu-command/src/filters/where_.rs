use nu_engine::eval_expression;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EvaluationContext};
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
        context: &EvaluationContext,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = call.positional[0].clone();

        let context = context.enter_scope();

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
                    context.add_var(var_id, value.clone());

                    let result = eval_expression(&context, &cond);

                    match result {
                        Ok(result) => result.is_true(),
                        _ => false,
                    }
                })
                .into_pipeline_data()),
            PipelineData::Value(Value::List { vals, span }) => Ok(vals
                .into_iter()
                .filter(move |value| {
                    context.add_var(var_id, value.clone());

                    let result = eval_expression(&context, &cond);

                    match result {
                        Ok(result) => result.is_true(),
                        _ => false,
                    }
                })
                .into_pipeline_data()),
            PipelineData::Value(x) => {
                context.add_var(var_id, x.clone());

                let result = eval_expression(&context, &cond)?;

                if result.is_true() {
                    Ok(x.into_pipeline_data())
                } else {
                    Ok(PipelineData::new())
                }
            }
        }
    }
}
