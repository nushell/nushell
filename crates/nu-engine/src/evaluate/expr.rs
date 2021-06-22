use crate::evaluate_baseline_expr;

use log::{log_enabled, trace};

use crate::evaluation_context::EvaluationContext;
use nu_errors::ShellError;
use nu_protocol::hir::SpannedExpression;
use nu_protocol::{UntaggedValue, Value};
use nu_stream::{InputStream, IntoInputStream};

pub(crate) fn run_expression_block(
    expr: &SpannedExpression,
    ctx: &EvaluationContext,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::expr", "->");
        trace!(target: "nu::run::expr", "{:?}", expr);
    }

    let output = evaluate_baseline_expr(expr, ctx)?;

    match output {
        Value {
            value: UntaggedValue::Table(x),
            ..
        } => Ok(InputStream::from_stream(x.into_iter())),
        output => Ok(std::iter::once(Ok(output)).into_input_stream()),
    }
}
