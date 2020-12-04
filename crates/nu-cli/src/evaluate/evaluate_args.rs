// TODO: Temporary redirect
use crate::evaluate::evaluate_baseline_expr;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{hir, EvaluatedArgs, Scope, UntaggedValue, Value};
use std::sync::Arc;

pub(crate) async fn evaluate_args(
    call: &hir::Call,
    scope: Arc<Scope>,
) -> Result<EvaluatedArgs, ShellError> {
    let mut positional_args: Vec<Value> = vec![];

    if let Some(positional) = &call.positional {
        for pos in positional {
            let result = evaluate_baseline_expr(pos, scope.clone()).await?;
            positional_args.push(result);
        }
    }

    let positional = if !positional_args.is_empty() {
        Some(positional_args)
    } else {
        None
    };

    let mut named_args = IndexMap::new();

    if let Some(named) = &call.named {
        for (name, value) in named.iter() {
            match value {
                hir::NamedValue::PresentSwitch(tag) => {
                    named_args.insert(name.clone(), UntaggedValue::boolean(true).into_value(tag));
                }
                hir::NamedValue::Value(_, expr) => {
                    named_args.insert(
                        name.clone(),
                        evaluate_baseline_expr(expr, scope.clone()).await?,
                    );
                }
                _ => {}
            };
        }
    }

    let named = if !named_args.is_empty() {
        Some(named_args)
    } else {
        None
    };

    Ok(EvaluatedArgs::new(positional, named))
}
