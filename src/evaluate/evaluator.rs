use crate::errors::Description;
use crate::object::base::Block;
use crate::parser::{
    hir::{self, Expression, RawExpression},
    CommandRegistry, Text,
};
use crate::prelude::*;
use derive_new::new;
use indexmap::IndexMap;

#[derive(new)]
pub struct Scope {
    it: Tagged<Value>,
    #[new(default)]
    vars: IndexMap<String, Tagged<Value>>,
}

impl Scope {
    pub(crate) fn empty() -> Scope {
        Scope {
            it: Value::nothing().tagged_unknown(),
            vars: IndexMap::new(),
        }
    }

    pub(crate) fn it_value(value: Tagged<Value>) -> Scope {
        Scope {
            it: value,
            vars: IndexMap::new(),
        }
    }
}

pub(crate) fn evaluate_baseline_expr(
    expr: &Expression,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<Tagged<Value>, ShellError> {
    match &expr.item {
        RawExpression::Literal(literal) => Ok(evaluate_literal(expr.copy_span(literal), source)),
        RawExpression::FilePath(path) => Ok(Value::path(path.clone()).tagged(expr.span())),
        RawExpression::Synthetic(hir::Synthetic::String(s)) => Ok(Value::string(s).tagged_unknown()),
        RawExpression::Variable(var) => evaluate_reference(var, scope, source),
        RawExpression::ExternalCommand(external) => evaluate_external(external, scope, source),
        RawExpression::Binary(binary) => {
            let left = evaluate_baseline_expr(binary.left(), registry, scope, source)?;
            let right = evaluate_baseline_expr(binary.right(), registry, scope, source)?;

            match left.compare(binary.op(), &*right) {
                Ok(result) => Ok(Tagged::from_simple_spanned_item(
                    Value::boolean(result),
                    expr.span(),
                )),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    binary.left().copy_span(left_type),
                    binary.right().copy_span(right_type),
                )),
            }
        }
        RawExpression::List(list) => {
            let mut exprs = vec![];

            for expr in list {
                let expr = evaluate_baseline_expr(expr, registry, scope, source)?;
                exprs.push(expr);
            }

            Ok(Value::List(exprs).tagged(Tag::unknown_origin(expr.span())))
        }
        RawExpression::Block(block) => Ok(Tagged::from_simple_spanned_item(
            Value::Block(Block::new(block.clone(), source.clone(), expr.span())),
            expr.span(),
        )),
        RawExpression::Path(path) => {
            let value = evaluate_baseline_expr(path.head(), registry, scope, source)?;
            let mut item = value;

            for name in path.tail() {
                let next = item.get_data_by_key(name);

                match next {
                    None => {
                        return Err(ShellError::missing_property(
                            Description::from(item.tagged_type_name()),
                            Description::from(name.clone()),
                        ))
                    }
                    Some(next) => {
                        item = Tagged::from_simple_spanned_item(
                            next.clone().item,
                            (expr.span().start, name.span().end),
                        )
                    }
                };
            }

            Ok(Tagged::from_simple_spanned_item(
                item.item().clone(),
                expr.span(),
            ))
        }
        RawExpression::Boolean(_boolean) => unimplemented!(),
    }
}

fn evaluate_literal(literal: Tagged<&hir::Literal>, source: &Text) -> Tagged<Value> {
    let result = match literal.item {
        hir::Literal::Number(int) => int.into(),
        hir::Literal::Size(int, unit) => unit.compute(int),
        hir::Literal::String(span) => Value::string(span.slice(source)),
        hir::Literal::Bare => Value::string(literal.span().slice(source)),
    };

    literal.map(|_| result)
}

fn evaluate_reference(
    name: &hir::Variable,
    scope: &Scope,
    source: &Text,
) -> Result<Tagged<Value>, ShellError> {
    match name {
        hir::Variable::It(span) => Ok(scope.it.item.clone().simple_spanned(span)),
        hir::Variable::Other(span) => Ok(scope
            .vars
            .get(span.slice(source))
            .map(|v| v.clone())
            .unwrap_or_else(|| Value::nothing().simple_spanned(span))),
    }
}

fn evaluate_external(
    external: &hir::ExternalCommand,
    _scope: &Scope,
    _source: &Text,
) -> Result<Tagged<Value>, ShellError> {
    Err(ShellError::syntax_error(
        "Unexpected external command".tagged(external.name()),
    ))
}
