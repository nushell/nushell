use crate::ParseError;
use nu_protocol::{
    ast::{Expr, Expression, Operator},
    engine::StateWorkingSet,
    Type,
};

pub fn type_compatible(lhs: &Type, rhs: &Type) -> bool {
    match (lhs, rhs) {
        (Type::List(c), Type::List(d)) => type_compatible(c, d),
        (Type::Unknown, _) => true,
        (_, Type::Unknown) => true,
        (lhs, rhs) => lhs == rhs,
    }
}

pub fn math_result_type(
    _working_set: &StateWorkingSet,
    lhs: &mut Expression,
    op: &mut Expression,
    rhs: &mut Expression,
) -> (Type, Option<ParseError>) {
    //println!("checking: {:?} {:?} {:?}", lhs, op, rhs);
    match &op.expr {
        Expr::Operator(operator) => match operator {
            Operator::Plus => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::String, Type::String) => (Type::String, None),
                (Type::Duration, Type::Duration) => (Type::Duration, None),
                (Type::Filesize, Type::Filesize) => (Type::Filesize, None),

                (Type::Unknown, _) => (Type::Unknown, None),
                (_, Type::Unknown) => (Type::Unknown, None),
                (Type::Int, _) => {
                    let ty = rhs.ty.clone();
                    *rhs = Expression::garbage(rhs.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            ty,
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Minus => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Duration, Type::Duration) => (Type::Duration, None),
                (Type::Filesize, Type::Filesize) => (Type::Filesize, None),

                (Type::Unknown, _) => (Type::Unknown, None),
                (_, Type::Unknown) => (Type::Unknown, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Multiply => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),

                (Type::Unknown, _) => (Type::Unknown, None),
                (_, Type::Unknown) => (Type::Unknown, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Divide => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),

                (Type::Unknown, _) => (Type::Unknown, None),
                (_, Type::Unknown) => (Type::Unknown, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::LessThan => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::LessThanOrEqual => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::GreaterThan => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::GreaterThanOrEqual => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Equal => match (&lhs.ty, &rhs.ty) {
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),

                (x, y) if x == y => (Type::Bool, None),
                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::NotEqual => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::In => match (&lhs.ty, &rhs.ty) {
                (t, Type::List(u)) if type_compatible(t, u) => (Type::Bool, None),
                (Type::Int | Type::Float, Type::Range) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::String, Type::Record(_, _)) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::NotIn => match (&lhs.ty, &rhs.ty) {
                (t, Type::List(u)) if type_compatible(t, u) => (Type::Bool, None),
                (Type::Int | Type::Float, Type::Range) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::String, Type::Record(_, _)) => (Type::Bool, None),

                (Type::Unknown, _) => (Type::Bool, None),
                (_, Type::Unknown) => (Type::Bool, None),
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::UnsupportedOperation(
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
            },

            _ => {
                *op = Expression::garbage(op.span);

                (
                    Type::Unknown,
                    Some(ParseError::UnsupportedOperation(
                        op.span,
                        lhs.span,
                        lhs.ty.clone(),
                        rhs.span,
                        rhs.ty.clone(),
                    )),
                )
            }
        },
        _ => {
            *op = Expression::garbage(op.span);

            (
                Type::Unknown,
                Some(ParseError::IncompleteMathExpression(op.span)),
            )
        }
    }
}
