// TODO: Temporary redirect
use crate::evaluate::evaluator::evaluate_baseline_expr;
use crate::evaluation_context::EvaluationContext;
use indexmap::IndexMap;
use nu_errors::ShellError;
use nu_protocol::{hir, EvaluatedArgs, UntaggedValue, Value};

pub(crate) fn evaluate_args(
    call: &hir::Call,
    ctx: &EvaluationContext,
) -> Result<EvaluatedArgs, ShellError> {
    let mut positional_args: Vec<Value> = vec![];

    if let Some(positional) = &call.positional {
        for pos in positional {
            let result = evaluate_baseline_expr(pos, ctx)?;
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
        for (name, value) in named {
            match value {
                hir::NamedValue::PresentSwitch(tag) => {
                    named_args.insert(name.clone(), UntaggedValue::boolean(true).into_value(tag));
                }
                hir::NamedValue::Value(_, expr) => {
                    named_args.insert(name.clone(), evaluate_baseline_expr(expr, ctx)?);
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
