use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;

use log::{log_enabled, trace};

use nu_errors::ShellError;
use nu_protocol::hir::SpannedExpression;
use nu_protocol::Scope;

pub(crate) fn run_expression_block(
    expr: SpannedExpression,
    context: &mut Context,
    input: InputStream,
    scope: &Scope,
) -> Result<InputStream, ShellError> {
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::expr", "->");
        trace!(target: "nu::run::expr", "{:?}", expr);
    }

    let scope = scope.clone();
    let registry = context.registry().clone();
    let stream = input.map(move |row| {
        let scope = scope.clone().set_it(row);
        evaluate_baseline_expr(&expr, &registry, &scope)
    });

    Ok(stream.to_input_stream())
}
