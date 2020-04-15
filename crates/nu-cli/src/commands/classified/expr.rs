use crate::evaluate::evaluate_baseline_expr;
use crate::prelude::*;
use log::{log_enabled, trace};
use nu_errors::ShellError;
use nu_protocol::hir::SpannedExpression;

use futures_util::pin_mut;
use nu_protocol::Scope;

pub(crate) fn run_expression_block(
    expr: SpannedExpression,
    context: &mut Context,
    input: Option<InputStream>,
    scope: &Scope,
) -> Result<Option<InputStream>, ShellError> {
    let scope = scope.clone();
    if log_enabled!(log::Level::Trace) {
        trace!(target: "nu::run::expr", "->");
        trace!(target: "nu::run::expr", "{:?}", expr);
    }

    let registry = context.registry().clone();

    let stream = async_stream! {
        if let Some(input) = input {
            let values = input.values;
            pin_mut!(values);

            while let Some(row) = values.next().await {
                let scope = scope.clone().set_it(row);
                yield evaluate_baseline_expr(&expr, &registry, &scope);
            }
        } else {
            yield evaluate_baseline_expr(&expr, &registry, &scope);
        }
    };

    Ok(Some(stream.to_input_stream()))
}
