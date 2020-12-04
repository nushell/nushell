use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;

use log::{log_enabled, trace};

use futures::stream::once;
use nu_errors::ShellError;
use nu_protocol::hir::SpannedExpression;
use nu_protocol::Scope;

pub(crate) async fn run_expression_block(
    expr: SpannedExpression,
    context: &mut EvaluationContext,
    scope: Arc<Scope>,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::expr", "->");
        trace!(target: "nu::run::expr", "{:?}", expr);
    }

    let output = evaluate_baseline_expr(&expr, scope).await?;

    Ok(once(async { Ok(output) }).to_input_stream())
}
