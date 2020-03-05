// TODO: Temporary redirect
use crate::context::CommandRegistry;
use crate::evaluate::evaluate_baseline_expr;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_parser::hir;
use nu_protocol::{EvaluatedArgs, Scope, UntaggedValue, Value};
use nu_source::Text;

pub(crate) fn evaluate_args(
    call: &hir::Call,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<EvaluatedArgs, ShellError> {
    let positional: Result<Option<Vec<_>>, _> = call
        .positional
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|e| evaluate_baseline_expr(e, registry, scope, source))
                .collect()
        })
        .transpose();

    let positional = positional?;

    let named: Result<Option<IndexMap<String, Value>>, ShellError> = call
        .named
        .as_ref()
        .map(|n| {
            let mut results = IndexMap::new();

            for (name, value) in n.named.iter() {
                match value {
                    hir::NamedValue::PresentSwitch(tag) => {
                        results.insert(name.clone(), UntaggedValue::boolean(true).into_value(tag));
                    }
                    hir::NamedValue::Value(expr) => {
                        results.insert(
                            name.clone(),
                            evaluate_baseline_expr(expr, registry, scope, source)?,
                        );
                    }

                    _ => {}
                };
            }

            Ok(results)
        })
        .transpose();

    let named = named?;

    Ok(EvaluatedArgs::new(positional, named))
}
