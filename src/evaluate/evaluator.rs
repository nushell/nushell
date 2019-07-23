use crate::errors::Description;
use crate::object::base::Block;
use crate::parser::{
    hir::{self, Expression, RawExpression},
    CommandRegistry, Spanned, Text,
};
use crate::prelude::*;
use derive_new::new;
use indexmap::IndexMap;

#[derive(new)]
pub struct Scope {
    it: Spanned<Value>,
    #[new(default)]
    vars: IndexMap<String, Spanned<Value>>,
}

impl Scope {
    crate fn empty() -> Scope {
        Scope {
            it: Value::nothing().spanned_unknown(),
            vars: IndexMap::new(),
        }
    }

    crate fn it_value(value: Spanned<Value>) -> Scope {
        Scope {
            it: value,
            vars: IndexMap::new(),
        }
    }
}

crate fn evaluate_baseline_expr(
    expr: &Expression,
    registry: &CommandRegistry,
    scope: &Scope,
    source: &Text,
) -> Result<Spanned<Value>, ShellError> {
    match &expr.item {
        RawExpression::Literal(literal) => Ok(evaluate_literal(expr.copy_span(*literal), source)),
        RawExpression::Synthetic(hir::Synthetic::String(s)) => {
            Ok(Value::string(s).spanned_unknown())
        }
        RawExpression::Variable(var) => evaluate_reference(var, scope, source),
        RawExpression::Binary(binary) => {
            let left = evaluate_baseline_expr(binary.left(), registry, scope, source)?;
            let right = evaluate_baseline_expr(binary.right(), registry, scope, source)?;

            match left.compare(binary.op(), &*right) {
                Ok(result) => Ok(Spanned::from_item(Value::boolean(result), *expr.span())),
                Err((left_type, right_type)) => Err(ShellError::coerce_error(
                    binary.left().copy_span(left_type),
                    binary.right().copy_span(right_type),
                )),
            }
        }
        RawExpression::Block(block) => Ok(Spanned::from_item(
            Value::Block(Block::new(block.clone(), source.clone(), *expr.span())),
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
                            Description::from(item.spanned_type_name()),
                            Description::from(name.clone()),
                        ))
                    }
                    Some(next) => {
                        item = Spanned::from_item(
                            next.clone().item,
                            (expr.span().start, name.span().end),
                        )
                    }
                };
            }

            Ok(Spanned::from_item(item.item().clone(), expr.span()))
        }
        RawExpression::Boolean(_boolean) => unimplemented!(),
    }
}

fn evaluate_literal(literal: Spanned<hir::Literal>, source: &Text) -> Spanned<Value> {
    let result = match literal.item {
        hir::Literal::Integer(int) => Value::int(int),
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
) -> Result<Spanned<Value>, ShellError> {
    match name {
        hir::Variable::It(span) => Ok(Spanned::from_item(scope.it.item.clone(), span)),
        hir::Variable::Other(span) => Ok(scope
            .vars
            .get(span.slice(source))
            .map(|v| v.clone())
            .unwrap_or_else(|| Value::nothing().spanned(span))),
    }
}
