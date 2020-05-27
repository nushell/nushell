use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;

use log::{log_enabled, trace};

use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::hir::SpannedExpression;
use nu_protocol::Value;

pub(crate) async fn run_expression_block(
    expr: SpannedExpression,
    context: &mut Context,
    it: &Value,
    vars: &IndexMap<String, Value>,
    env: &IndexMap<String, String>,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::expr", "->");
        trace!(target: "nu::run::expr", "{:?}", expr);
    }

    let registry = context.registry().clone();
    let output = evaluate_baseline_expr(&expr, &registry, it, vars, env).await?;

    Ok(once(async { Ok(output) }).to_input_stream())
}
