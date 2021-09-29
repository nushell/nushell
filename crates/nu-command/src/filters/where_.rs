use nu_engine::eval_expression;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EvaluationContext};
use nu_protocol::{IntoValueStream, ShellError, Signature, SyntaxShape, Value};

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
        input: Value,
    ) -> Result<nu_protocol::Value, nu_protocol::ShellError> {
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
            Value::Stream { stream, span } => {
                let output_stream = stream
                    .filter(move |value| {
                        context.add_var(var_id, value.clone());

                        let result = eval_expression(&context, &cond);

                        match result {
                            Ok(result) => result.is_true(),
                            _ => false,
                        }
                    })
                    .into_value_stream();

                Ok(Value::Stream {
                    stream: output_stream,
                    span,
                })
            }
            Value::List { vals, span } => {
                let output_stream = vals
                    .into_iter()
                    .filter(move |value| {
                        context.add_var(var_id, value.clone());

                        let result = eval_expression(&context, &cond);

                        match result {
                            Ok(result) => result.is_true(),
                            _ => false,
                        }
                    })
                    .into_value_stream();

                Ok(Value::Stream {
                    stream: output_stream,
                    span,
                })
            }
            x => {
                context.add_var(var_id, x.clone());

                let result = eval_expression(&context, &cond)?;

                if result.is_true() {
                    Ok(x)
                } else {
                    Ok(Value::Nothing { span: call.head })
                }
            }
        }
    }
}
