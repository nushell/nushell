use crate::command_registry::CommandRegistry;
use crate::commands::classified::block::run_block;
use crate::did_you_mean;
use crate::evaluate::operator::apply_operator;
use crate::prelude::*;
use async_recursion::async_recursion;
use log::trace;
use nu_errors::{ArgumentError, ShellError};
use nu_protocol::hir::{self, Expression, ExternalRedirection, RangeOperator, SpannedExpression};
use nu_protocol::{
    ColumnPath, Primitive, RangeInclusion, Scope, UnspannedPathMember, UntaggedValue, Value,
};

#[async_recursion]
pub(crate) async fn evaluate_baseline_expr(
    expr: &SpannedExpression,
    registry: &CommandRegistry,
    scope: Arc<Scope>,
) -> Result<Value, ShellError> {
    let tag = Tag {
        span: expr.span,
        anchor: None,
    };
    let span = expr.span;
    match &expr.expr {
        Expression::Literal(literal) => Ok(evaluate_literal(&literal, span)),
        Expression::ExternalWord => Err(ShellError::argument_error(
            "Invalid external word".spanned(tag.span),
            ArgumentError::InvalidExternalWord,
        )),
        Expression::FilePath(path) => Ok(UntaggedValue::path(path.clone()).into_value(tag)),
        Expression::Synthetic(hir::Synthetic::String(s)) => {
            Ok(UntaggedValue::string(s).into_untagged_value())
        }
        Expression::Variable(var) => evaluate_reference(&var, scope, tag),
        Expression::Command => unimplemented!(),
        Expression::Invocation(block) => evaluate_invocation(block, registry, scope).await,
        Expression::ExternalCommand(_) => unimplemented!(),
        Expression::Binary(binary) => {
            // TODO: If we want to add short-circuiting, we'll need to move these down
            let left = evaluate_baseline_expr(&binary.left, registry, scope.clone()).await?;
            let right = evaluate_baseline_expr(&binary.right, registry, scope).await?;

            trace!("left={:?} right={:?}", left.value, right.value);

            match binary.op.expr {
                Expression::Literal(hir::Literal::Operator(op)) => {
                    match apply_operator(op, &left, &right) {
                        Ok(result) => match result {
                            UntaggedValue::Error(shell_err) => Err(shell_err),
                            _ => Ok(result.into_value(tag)),
                        },
                        Err((left_type, right_type)) => Err(ShellError::coerce_error(
                            left_type.spanned(binary.left.span),
                            right_type.spanned(binary.right.span),
                        )),
                    }
                }
                _ => unreachable!(),
            }
        }
        Expression::Range(range) => {
            let left = if let Some(left) = &range.left {
                evaluate_baseline_expr(&left, registry, scope.clone()).await?
            } else {
                Value::nothing()
            };

            let right = if let Some(right) = &range.right {
                evaluate_baseline_expr(&right, registry, scope).await?
            } else {
                Value::nothing()
            };

            let left_span = left.tag.span;
            let right_span = right.tag.span;

            let left = (
                left.as_primitive()?.spanned(left_span),
                RangeInclusion::Inclusive,
            );
            let right = (
                right.as_primitive()?.spanned(right_span),
                match &range.operator.item {
                    RangeOperator::Inclusive => RangeInclusion::Inclusive,
                    RangeOperator::RightExclusive => RangeInclusion::Exclusive,
                },
            );

            Ok(UntaggedValue::range(left, right).into_value(tag))
        }
        Expression::Table(headers, cells) => {
            let mut output_headers = vec![];

            for expr in headers {
                let val = evaluate_baseline_expr(&expr, registry, scope.clone()).await?;

                let header = val.as_string()?;
                output_headers.push(header);
            }

            let mut output_table = vec![];

            for row in cells {
                if row.len() != headers.len() {
                    match (row.first(), row.last()) {
                        (Some(first), Some(last)) => {
                            return Err(ShellError::labeled_error(
                                "Cell count doesn't match header count",
                                format!("expected {} columns", headers.len()),
                                Span::new(first.span.start(), last.span.end()),
                            ));
                        }
                        _ => {
                            return Err(ShellError::untagged_runtime_error(
                                "Cell count doesn't match header count",
                            ));
                        }
                    }
                }

                let mut row_output = IndexMap::new();
                for cell in output_headers.iter().zip(row.iter()) {
                    let val = evaluate_baseline_expr(&cell.1, registry, scope.clone()).await?;
                    row_output.insert(cell.0.clone(), val);
                }
                output_table.push(UntaggedValue::row(row_output).into_value(tag.clone()));
            }

            Ok(UntaggedValue::Table(output_table).into_value(tag))
        }
        Expression::List(list) => {
            let mut exprs = vec![];

            for expr in list {
                let expr = evaluate_baseline_expr(&expr, registry, scope.clone()).await?;
                exprs.push(expr);
            }

            Ok(UntaggedValue::Table(exprs).into_value(tag))
        }
        Expression::Block(block) => Ok(UntaggedValue::Block(block.clone()).into_value(&tag)),
        Expression::Path(path) => {
            let value = evaluate_baseline_expr(&path.head, registry, scope).await?;
            let mut item = value;

            for member in &path.tail {
                let next = item.get_data_by_member(member);

                match next {
                    Err(err) => {
                        if let UnspannedPathMember::String(_name) = &member.unspanned {
                            let possible_matches = did_you_mean(&item, member.as_string());

                            match possible_matches {
                                Some(p) => {
                                    return Err(ShellError::labeled_error(
                                        "Unknown column",
                                        format!("did you mean '{}'?", p[0]),
                                        &member.span,
                                    ));
                                }
                                None => return Err(err),
                            }
                        }
                    }
                    Ok(next) => {
                        item = next.clone().value.into_value(&tag);
                    }
                };
            }

            Ok(item.value.into_value(tag))
        }
        Expression::Boolean(_boolean) => unimplemented!(),
        Expression::Garbage => unimplemented!(),
    }
}

fn evaluate_literal(literal: &hir::Literal, span: Span) -> Value {
    match &literal {
        hir::Literal::ColumnPath(path) => {
            let members = path.iter().map(|member| member.to_path_member()).collect();

            UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::new(members)))
                .into_value(span)
        }
        hir::Literal::Number(int) => match int {
            nu_protocol::hir::Number::Int(i) => UntaggedValue::int(i.clone()).into_value(span),
            nu_protocol::hir::Number::Decimal(d) => {
                UntaggedValue::decimal(d.clone()).into_value(span)
            }
        },
        hir::Literal::Size(int, unit) => unit.compute(&int).into_value(span),
        hir::Literal::String(string) => UntaggedValue::string(string).into_value(span),
        hir::Literal::GlobPattern(pattern) => UntaggedValue::pattern(pattern).into_value(span),
        hir::Literal::Bare(bare) => UntaggedValue::string(bare.clone()).into_value(span),
        hir::Literal::Operator(_) => unimplemented!("Not sure what to do with operator yet"),
    }
}

fn evaluate_reference(
    name: &hir::Variable,
    scope: Arc<Scope>,
    tag: Tag,
) -> Result<Value, ShellError> {
    match name {
        hir::Variable::It(_) => match scope.it() {
            Some(v) => Ok(v),
            None => Err(ShellError::labeled_error(
                "$it variable not in scope",
                "not in scope (are you missing an 'each'?)",
                tag.span,
            )),
        },
        hir::Variable::Other(name, _) => match name {
            x if x == "$nu" => crate::evaluate::variables::nu(&scope.env(), tag),
            x if x == "$true" => Ok(Value {
                value: UntaggedValue::boolean(true),
                tag,
            }),
            x if x == "$false" => Ok(Value {
                value: UntaggedValue::boolean(false),
                tag,
            }),
            x => match scope.var(x) {
                Some(v) => Ok(v),
                None => Err(ShellError::labeled_error(
                    "Variable not in scope",
                    "unknown variable",
                    tag.span,
                )),
            },
        },
    }
}

async fn evaluate_invocation(
    block: &hir::Block,
    registry: &CommandRegistry,
    scope: Arc<Scope>,
) -> Result<Value, ShellError> {
    // FIXME: we should use a real context here
    let mut context = EvaluationContext::basic()?;
    context.registry = registry.clone();

    let input = match scope.it() {
        Some(it) => InputStream::one(it),
        None => InputStream::empty(),
    };

    let mut block = block.clone();
    block.set_redirect(ExternalRedirection::Stdout);

    let result = run_block(&block, &mut context, input, scope).await?;

    let output = result.into_vec().await;

    if let Some(e) = context.get_errors().get(0) {
        return Err(e.clone());
    }

    match output.len() {
        x if x > 1 => Ok(UntaggedValue::Table(output).into_value(Tag::unknown())),
        1 => Ok(output[0].clone()),
        _ => Ok(UntaggedValue::nothing().into_value(Tag::unknown())),
    }
}
