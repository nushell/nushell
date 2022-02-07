use crate::evaluate::block::run_block;
use crate::evaluate::operator::apply_operator;
use crate::evaluation_context::EvaluationContext;
use indexmap::IndexMap;
use log::trace;
use nu_errors::{ArgumentError, ShellError};
use nu_protocol::did_you_mean;
use nu_protocol::{
    hir::{self, CapturedBlock, Expression, RangeOperator, SpannedExpression},
    Dictionary,
};
use nu_protocol::{
    ColumnPath, Primitive, RangeInclusion, UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::{Span, SpannedItem, Tag};
use nu_stream::InputStream;
use nu_value_ext::ValueExt;

pub fn evaluate_baseline_expr(
    expr: &SpannedExpression,
    ctx: &EvaluationContext,
) -> Result<Value, ShellError> {
    let tag = Tag {
        span: expr.span,
        anchor: None,
    };
    let span = expr.span;
    match &expr.expr {
        Expression::Literal(literal) => Ok(evaluate_literal(literal, span)),
        Expression::ExternalWord => Err(ShellError::argument_error(
            "Invalid external word".spanned(tag.span),
            ArgumentError::InvalidExternalWord,
        )),
        Expression::FilePath(path) => Ok(UntaggedValue::filepath(path.clone()).into_value(tag)),
        Expression::Synthetic(hir::Synthetic::String(s)) => {
            Ok(UntaggedValue::string(s).into_untagged_value())
        }
        expr @ Expression::Variable(_, _) => evaluate_reference(&Variable::from(expr), ctx, span),
        Expression::Command => unimplemented!(),
        Expression::Subexpression(block) => evaluate_subexpression(block, ctx),
        Expression::ExternalCommand(_) => unimplemented!(),
        Expression::Binary(binary) => {
            // TODO: If we want to add short-circuiting, we'll need to move these down
            let left = evaluate_baseline_expr(&binary.left, ctx)?;
            let right = evaluate_baseline_expr(&binary.right, ctx)?;

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
                _ => Err(ShellError::labeled_error(
                    "Unknown operator",
                    "unknown operator",
                    binary.op.span,
                )),
            }
        }
        Expression::Range(range) => {
            let left = if let Some(left) = &range.left {
                evaluate_baseline_expr(left, ctx)?
            } else {
                Value::nothing()
            };

            let right = if let Some(right) = &range.right {
                evaluate_baseline_expr(right, ctx)?
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
                let val = evaluate_baseline_expr(expr, ctx)?;

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
                for cell in output_headers.iter().zip(row) {
                    let val = evaluate_baseline_expr(cell.1, ctx)?;
                    row_output.insert(cell.0.clone(), val);
                }
                output_table.push(UntaggedValue::row(row_output).into_value(tag.clone()));
            }

            Ok(UntaggedValue::Table(output_table).into_value(tag))
        }
        Expression::List(list) => {
            let mut exprs = vec![];

            for expr in list {
                let expr = evaluate_baseline_expr(expr, ctx)?;
                exprs.push(expr);
            }

            Ok(UntaggedValue::Table(exprs).into_value(tag))
        }
        Expression::Block(block) => {
            // Capture the current values of all free variables
            let mut known_variables = vec![];
            let free_variables = block.get_free_variables(&mut known_variables);

            let mut captured = Dictionary::new(IndexMap::new());
            for free_variable in &free_variables {
                if let Some(v) = ctx.scope.get_var(free_variable) {
                    captured.insert(free_variable.into(), v.clone());
                }
            }

            Ok(
                UntaggedValue::Block(Box::new(CapturedBlock::new(block.clone(), captured)))
                    .into_value(&tag),
            )
        }
        Expression::FullColumnPath(path) => {
            let value = evaluate_baseline_expr(&path.head, ctx)?;
            let mut item = value;

            for member in &path.tail {
                let next = item.get_data_by_member(member);

                match next {
                    Err(err) => match &member.unspanned {
                        UnspannedPathMember::String(_name) => {
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
                        UnspannedPathMember::Int(_row) => {
                            return Err(ShellError::labeled_error(
                                "Unknown row",
                                "unknown row",
                                &member.span,
                            ));
                        }
                    },
                    Ok(next) => {
                        item = next.clone().value.into_value(&tag);
                    }
                };
            }

            if path.tail.is_empty() {
                Ok(item)
            } else {
                Ok(item.value.into_value(tag))
            }
        }
        Expression::Boolean(_boolean) => Ok(UntaggedValue::boolean(*_boolean).into_value(tag)),
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
            nu_protocol::hir::Number::BigInt(i) => {
                UntaggedValue::big_int(i.clone()).into_value(span)
            }
            nu_protocol::hir::Number::Int(i) => UntaggedValue::int(*i).into_value(span),
            nu_protocol::hir::Number::Decimal(d) => {
                UntaggedValue::decimal(d.clone()).into_value(span)
            }
        },
        hir::Literal::Size(int, unit) => unit.compute(int).into_value(span),
        hir::Literal::String(string) => UntaggedValue::string(string).into_value(span),
        hir::Literal::GlobPattern(pattern) => UntaggedValue::glob_pattern(pattern).into_value(span),
        hir::Literal::Bare(bare) => UntaggedValue::string(bare.clone()).into_value(span),
        hir::Literal::Operator(_) => unimplemented!("Not sure what to do with operator yet"),
    }
}

pub enum Variable<'a> {
    Nu,
    Scope,
    True,
    False,
    Nothing,
    Other(&'a str),
}

impl<'a> Variable<'a> {
    pub fn list() -> Vec<String> {
        vec![
            String::from("$nu"),
            String::from("$scope"),
            String::from("$true"),
            String::from("$false"),
            String::from("$nothing"),
        ]
    }
}

impl<'a> From<&'a Expression> for Variable<'a> {
    fn from(expr: &'a Expression) -> Self {
        match &expr {
            Expression::Variable(name, _) => match name.as_str() {
                "$nu" => Self::Nu,
                "$scope" => Self::Scope,
                "$true" => Self::True,
                "$false" => Self::False,
                "$nothing" => Self::Nothing,
                _ => Self::Other(name),
            },
            _ => unreachable!(),
        }
    }
}

pub fn evaluate_reference(
    variable: &Variable,
    ctx: &EvaluationContext,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    match variable {
        Variable::Nu => crate::evaluate::variables::nu(&ctx.scope, ctx),
        Variable::Scope => crate::evaluate::variables::scope(
            &ctx.scope.get_aliases(),
            &ctx.scope.get_commands(),
            &ctx.scope.get_vars(),
        ),
        Variable::True => Ok(UntaggedValue::boolean(true).into_untagged_value()),
        Variable::False => Ok(UntaggedValue::boolean(false).into_untagged_value()),
        Variable::Nothing => Ok(UntaggedValue::nothing().into_untagged_value()),
        Variable::Other(name) => match ctx.scope.get_var(name) {
            Some(v) => Ok(v),
            None => Err(ShellError::labeled_error(
                "Variable not in scope",
                format!("unknown variable: {}", name),
                tag.into(),
            )),
        },
    }
}

fn evaluate_subexpression(
    block: &hir::Block,
    ctx: &EvaluationContext,
) -> Result<Value, ShellError> {
    // FIXME: we should use a real context here
    let input = match ctx.scope.get_var("$it") {
        Some(it) => InputStream::one(it),
        None => InputStream::empty(),
    };

    let result = run_block(block, ctx, input, hir::ExternalRedirection::Stdout)?;

    let output = result.into_vec();

    if let Some(e) = ctx.get_errors().get(0) {
        return Err(e.clone());
    }

    match output.len() {
        x if x > 1 => {
            let tag = output[0].tag.clone();
            Ok(UntaggedValue::Table(output).into_value(tag))
        }
        1 => Ok(output[0].clone()),
        _ => Ok(UntaggedValue::nothing().into_value(Tag::unknown())),
    }
}
