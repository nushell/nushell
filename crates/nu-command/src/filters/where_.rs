use nu_engine::eval_expression;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

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
        Signature::build("where")
            .required("cond", SyntaxShape::RowCondition, "condition")
            .category(Category::Filters)
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        let cond = call.positional[0].clone();

        let ctrlc = engine_state.ctrlc.clone();
        let engine_state = engine_state.clone();

        // FIXME: expensive
        let mut stack = stack.clone();

        let (var_id, cond) = match cond {
            Expression {
                expr: Expr::RowCondition(var_id, expr),
                ..
            } => (var_id, expr),
            _ => return Err(ShellError::InternalError("Expected row condition".into())),
        };

        input.filter(
            move |value| {
                stack.add_var(var_id, value.clone());

                let result = eval_expression(&engine_state, &mut stack, &cond);

                match result {
                    Ok(result) => result.is_true(),
                    _ => false,
                }
            },
            ctrlc,
        )
    }
}
