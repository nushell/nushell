use crate::context::CommandRegistry;
use crate::data::base::Block;
use crate::evaluate::operator::apply_operator;
use crate::prelude::*;
use log::trace;
use nu_errors::{ArgumentError, ShellError};
use nu_parser::hir::{self, Expression, SpannedExpression};
use nu_protocol::{
    ColumnPath, Evaluate, Primitive, RangeInclusion, Scope, UnspannedPathMember, UntaggedValue,
    Value,
};
use nu_source::Text;

pub(crate) fn evaluate_baseline_expr(
    expr: &SpannedExpression,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<Value, ShellError> {
    let tag = Tag {
        span: expr.span,
        anchor: None,
    };
    match &expr.expr {
        Expression::Literal(literal) => Ok(evaluate_literal(literal, expr.span, source)),
        Expression::ExternalWord => Err(ShellError::argument_error(
            "Invalid external word".spanned(tag.span),
            ArgumentError::InvalidExternalWord,
        )),
        Expression::FilePath(path) => Ok(UntaggedValue::path(path.clone()).into_value(tag)),
        Expression::Synthetic(hir::Synthetic::String(s)) => {
            Ok(UntaggedValue::string(s).into_untagged_value())
        }
        Expression::Variable(var) => evaluate_reference(var, scope, source, tag),
        Expression::Command(_) => evaluate_command(tag, scope, source),
        Expression::ExternalCommand(external) => evaluate_external(external, scope, source),
        Expression::Binary(binary) => {
            let left = evaluate_baseline_expr(&binary.left, registry, scope, source)?;
            let right = evaluate_baseline_expr(&binary.right, registry, scope, source)?;

            trace!("left={:?} right={:?}", left.value, right.value);

            match binary.op.expr {
                Expression::Literal(hir::Literal::Operator(op)) => {
                    match apply_operator(op, &left, &right) {
                        Ok(result) => Ok(result.into_value(tag)),
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
            let left = &range.left;
            let right = &range.right;

            let left = evaluate_baseline_expr(left, registry, scope, source)?;
            let right = evaluate_baseline_expr(right, registry, scope, source)?;
            let left_span = left.tag.span;
            let right_span = right.tag.span;

            let left = (
                left.as_primitive()?.spanned(left_span),
                RangeInclusion::Inclusive,
            );
            let right = (
                right.as_primitive()?.spanned(right_span),
                RangeInclusion::Exclusive,
            );

            Ok(UntaggedValue::range(left, right).into_value(tag))
        }
        Expression::List(list) => {
            let mut exprs = vec![];

            for expr in list {
                let expr = evaluate_baseline_expr(expr, registry, scope, source)?;
                exprs.push(expr);
            }

            Ok(UntaggedValue::Table(exprs).into_value(tag))
        }
        Expression::Block(block) => Ok(UntaggedValue::Block(Evaluate::new(Block::new(
            block.clone(),
            source.clone(),
            tag.clone(),
        )))
        .into_value(&tag)),
        Expression::Path(path) => {
            let value = evaluate_baseline_expr(&path.head, registry, scope, source)?;
            let mut item = value;

            for member in &path.tail {
                let next = item.get_data_by_member(member);

                match next {
                    Err(err) => {
                        let possibilities = item.data_descriptors();

                        if let UnspannedPathMember::String(name) = &member.unspanned {
                            let mut possible_matches: Vec<_> = possibilities
                                .iter()
                                .map(|x| (natural::distance::levenshtein_distance(x, &name), x))
                                .collect();

                            possible_matches.sort();

                            if !possible_matches.is_empty() {
                                return Err(ShellError::labeled_error(
                                    "Unknown column",
                                    format!("did you mean '{}'?", possible_matches[0].1),
                                    &tag,
                                ));
                            } else {
                                return Err(err);
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

fn evaluate_literal(literal: &hir::Literal, span: Span, source: &Text) -> Value {
    match &literal {
        hir::Literal::ColumnPath(path) => {
            let members = path.iter().map(|member| member.to_path_member()).collect();

            UntaggedValue::Primitive(Primitive::ColumnPath(ColumnPath::new(members)))
                .into_value(span)
        }
        hir::Literal::Number(int) => match int {
            nu_parser::hir::Number::Int(i) => UntaggedValue::int(i.clone()).into_value(span),
            nu_parser::hir::Number::Decimal(d) => {
                UntaggedValue::decimal(d.clone()).into_value(span)
            }
        },
        hir::Literal::Size(int, unit) => unit.compute(&int).into_value(span),
        hir::Literal::String(string) => UntaggedValue::string(string).into_value(span),
        hir::Literal::GlobPattern(pattern) => UntaggedValue::pattern(pattern).into_value(span),
        hir::Literal::Bare => UntaggedValue::string(span.slice(source)).into_value(span),
        hir::Literal::Operator(_) => unimplemented!("Not sure what to do with operator yet"),
    }
}

fn evaluate_reference(
    name: &hir::Variable,
    scope: &Scope,
    source: &Text,
    tag: Tag,
) -> Result<Value, ShellError> {
    trace!("Evaluating {:?} with Scope {:?}", name, scope);
    match name {
        hir::Variable::It(_) => Ok(scope.it.value.clone().into_value(tag)),
        hir::Variable::Other(_, span) => match span.slice(source) {
            x if x == "$nu" => crate::evaluate::variables::nu(tag),
            x => Ok(scope
                .vars
                .get(x)
                .cloned()
                .unwrap_or_else(|| UntaggedValue::nothing().into_value(tag))),
        },
    }
}

fn evaluate_external(
    external: &hir::ExternalCommand,
    _scope: &Scope,
    _source: &Text,
) -> Result<Value, ShellError> {
    Err(ShellError::syntax_error(
        "Unexpected external command".spanned(external.name.span),
    ))
}

fn evaluate_command(tag: Tag, _scope: &Scope, _source: &Text) -> Result<Value, ShellError> {
    Err(ShellError::syntax_error(
        "Unexpected command".spanned(tag.span),
    ))
}
