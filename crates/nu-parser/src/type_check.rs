use nu_protocol::{
    ast::{
        Assignment, Bits, Block, Boolean, Comparison, Expr, Expression, Math, Operator, Pipeline,
        Range,
    },
    engine::StateWorkingSet,
    ParseError, Type,
};

pub fn type_compatible(lhs: &Type, rhs: &Type) -> bool {
    // Structural subtyping
    let is_compatible = |expected: &[(String, Type)], found: &[(String, Type)]| {
        if expected.is_empty() || found.is_empty() {
            // We treat an incoming empty table/record type as compatible for typechecking purposes
            // It is the responsibility of the runtime to reject if necessary
            true
        } else if expected.len() > found.len() {
            false
        } else {
            expected.iter().all(|(col_x, ty_x)| {
                if let Some((_, ty_y)) = found.iter().find(|(col_y, _)| col_x == col_y) {
                    type_compatible(ty_x, ty_y)
                } else {
                    false
                }
            })
        }
    };

    match (lhs, rhs) {
        (Type::List(c), Type::List(d)) => type_compatible(c, d),
        (Type::List(c), Type::Table(table_fields)) => {
            if matches!(**c, Type::Any) {
                return true;
            }

            if let Type::Record(fields) = &**c {
                is_compatible(fields, table_fields)
            } else {
                false
            }
        }
        (Type::Table(table_fields), Type::List(c)) => {
            if matches!(**c, Type::Any) {
                return true;
            }

            if let Type::Record(fields) = &**c {
                is_compatible(table_fields, fields)
            } else {
                false
            }
        }
        (Type::Number, Type::Int) => true,
        (Type::Int, Type::Number) => true,
        (Type::Number, Type::Float) => true,
        (Type::Float, Type::Number) => true,
        (Type::Closure, Type::Block) => true,
        (Type::Any, _) => true,
        (_, Type::Any) => true,
        (Type::Record(lhs), Type::Record(rhs)) | (Type::Table(lhs), Type::Table(rhs)) => {
            is_compatible(lhs, rhs)
        }
        (Type::Glob, Type::String) => true,
        (lhs, rhs) => lhs == rhs,
    }
}

pub fn math_result_type(
    working_set: &mut StateWorkingSet,
    lhs: &mut Expression,
    op: &mut Expression,
    rhs: &mut Expression,
) -> (Type, Option<ParseError>) {
    match &op.expr {
        Expr::Operator(operator) => match operator {
            Operator::Math(Math::Plus) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Int) => (Type::Number, None),
                (Type::Int, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Float) => (Type::Number, None),
                (Type::Float, Type::Number) => (Type::Number, None),
                (Type::String, Type::String) => (Type::String, None),
                (Type::Date, Type::Duration) => (Type::Date, None),
                (Type::Duration, Type::Date) => (Type::Date, None),
                (Type::Duration, Type::Duration) => (Type::Duration, None),
                (Type::Filesize, Type::Filesize) => (Type::Filesize, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (
                    Type::Int
                    | Type::Float
                    | Type::String
                    | Type::Date
                    | Type::Duration
                    | Type::Filesize,
                    _,
                ) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "addition".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "addition".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Math(Math::Concat) => check_concat(working_set, lhs, rhs, op),
            Operator::Math(Math::Minus) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Int) => (Type::Number, None),
                (Type::Int, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Float) => (Type::Number, None),
                (Type::Float, Type::Number) => (Type::Number, None),
                (Type::Date, Type::Date) => (Type::Duration, None),
                (Type::Date, Type::Duration) => (Type::Date, None),
                (Type::Duration, Type::Duration) => (Type::Duration, None),
                (Type::Filesize, Type::Filesize) => (Type::Filesize, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int | Type::Float | Type::Date | Type::Duration | Type::Filesize, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "subtraction".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "subtraction".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Math(Math::Multiply) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Int) => (Type::Number, None),
                (Type::Int, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Float) => (Type::Number, None),
                (Type::Float, Type::Number) => (Type::Number, None),
                (Type::Filesize, Type::Int) => (Type::Filesize, None),
                (Type::Int, Type::Filesize) => (Type::Filesize, None),
                (Type::Filesize, Type::Float) => (Type::Filesize, None),
                (Type::Float, Type::Filesize) => (Type::Filesize, None),
                (Type::Duration, Type::Int) => (Type::Duration, None),
                (Type::Int, Type::Duration) => (Type::Duration, None),
                (Type::Duration, Type::Float) => (Type::Duration, None),
                (Type::Float, Type::Duration) => (Type::Duration, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int, _)
                | (Type::Float, _)
                | (Type::String, _)
                | (Type::Date, _)
                | (Type::Duration, _)
                | (Type::Filesize, _)
                | (Type::List(_), _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "multiplication".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "multiplication".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Math(Math::Pow) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Int) => (Type::Number, None),
                (Type::Int, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Float) => (Type::Number, None),
                (Type::Float, Type::Number) => (Type::Number, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int | Type::Float, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "exponentiation".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "exponentiation".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Math(Math::Divide) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Float, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Float, None),
                (Type::Number, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Number) => (Type::Float, None),
                (Type::Number, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Number) => (Type::Float, None),
                (Type::Filesize, Type::Filesize) => (Type::Float, None),
                (Type::Filesize, Type::Int) => (Type::Filesize, None),
                (Type::Filesize, Type::Float) => (Type::Filesize, None),
                (Type::Duration, Type::Duration) => (Type::Float, None),
                (Type::Duration, Type::Int) => (Type::Duration, None),
                (Type::Duration, Type::Float) => (Type::Duration, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int | Type::Float | Type::Filesize | Type::Duration, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "division".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "division".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Math(Math::Modulo) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Int) => (Type::Number, None),
                (Type::Int, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Float) => (Type::Number, None),
                (Type::Float, Type::Number) => (Type::Number, None),
                (Type::Filesize, Type::Filesize) => (Type::Filesize, None),
                (Type::Filesize, Type::Int) => (Type::Filesize, None),
                (Type::Filesize, Type::Float) => (Type::Filesize, None),
                (Type::Duration, Type::Duration) => (Type::Duration, None),
                (Type::Duration, Type::Int) => (Type::Duration, None),
                (Type::Duration, Type::Float) => (Type::Duration, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int | Type::Float | Type::Filesize | Type::Duration, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "division".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "division".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Math(Math::FloorDivision) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),
                (Type::Float, Type::Int) => (Type::Float, None),
                (Type::Int, Type::Float) => (Type::Float, None),
                (Type::Float, Type::Float) => (Type::Float, None),
                (Type::Number, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Int) => (Type::Number, None),
                (Type::Int, Type::Number) => (Type::Number, None),
                (Type::Number, Type::Float) => (Type::Number, None),
                (Type::Float, Type::Number) => (Type::Number, None),
                (Type::Filesize, Type::Filesize) => (Type::Int, None),
                (Type::Filesize, Type::Int) => (Type::Filesize, None),
                (Type::Filesize, Type::Float) => (Type::Filesize, None),
                (Type::Duration, Type::Duration) => (Type::Int, None),
                (Type::Duration, Type::Int) => (Type::Duration, None),
                (Type::Duration, Type::Float) => (Type::Duration, None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int | Type::Float | Type::Filesize | Type::Duration, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "floor division".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "floor division".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Boolean(Boolean::And)
            | Operator::Boolean(Boolean::Or)
            | Operator::Boolean(Boolean::Xor) => {
                match (&lhs.ty, &rhs.ty) {
                    (Type::Bool, Type::Bool) => (Type::Bool, None),

                    (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                    (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                    (Type::Any, _) => (Type::Any, None),
                    (_, Type::Any) => (Type::Any, None),

                    // FIX ME. This is added because there is no type output for custom function
                    // definitions. As soon as that syntax is added this should be removed
                    (a, b) if a == b => (Type::Bool, None),
                    (Type::Bool, _) => {
                        *op = Expression::garbage(working_set, op.span);
                        (
                            Type::Any,
                            Some(ParseError::UnsupportedOperationRHS(
                                "boolean operation".into(),
                                op.span,
                                lhs.span,
                                lhs.ty.clone(),
                                rhs.span,
                                rhs.ty.clone(),
                            )),
                        )
                    }
                    _ => {
                        *op = Expression::garbage(working_set, op.span);
                        (
                            Type::Any,
                            Some(ParseError::UnsupportedOperationLHS(
                                "boolean operation".into(),
                                op.span,
                                lhs.span,
                                lhs.ty.clone(),
                            )),
                        )
                    }
                }
            }
            Operator::Comparison(Comparison::LessThan) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Number, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Number) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Date, Type::Date) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),
                (Type::Bool, Type::Bool) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Nothing, _) => (Type::Nothing, None),
                (_, Type::Nothing) => (Type::Nothing, None),

                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),
                (Type::Int | Type::Float | Type::Duration | Type::Filesize, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "less-than comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "less-than comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::LessThanOrEqual) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Number, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Number) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Date, Type::Date) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),
                (Type::Bool, Type::Bool) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Nothing, _) => (Type::Nothing, None),
                (_, Type::Nothing) => (Type::Nothing, None),

                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),
                (Type::Int | Type::Float | Type::Duration | Type::Filesize, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "less-than or equal comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "less-than or equal comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::GreaterThan) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Number, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Number) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Date, Type::Date) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),
                (Type::Bool, Type::Bool) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),

                (Type::Nothing, _) => (Type::Nothing, None),
                (_, Type::Nothing) => (Type::Nothing, None),
                (Type::Int | Type::Float | Type::Duration | Type::Filesize, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "greater-than comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "greater-than comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::GreaterThanOrEqual) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Bool, None),
                (Type::Float, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Float) => (Type::Bool, None),
                (Type::Number, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Int) => (Type::Bool, None),
                (Type::Int, Type::Number) => (Type::Bool, None),
                (Type::Number, Type::Float) => (Type::Bool, None),
                (Type::Float, Type::Number) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Duration, Type::Duration) => (Type::Bool, None),
                (Type::Date, Type::Date) => (Type::Bool, None),
                (Type::Filesize, Type::Filesize) => (Type::Bool, None),
                (Type::Bool, Type::Bool) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),

                (Type::Nothing, _) => (Type::Nothing, None),
                (_, Type::Nothing) => (Type::Nothing, None),
                (Type::Int | Type::Float | Type::Duration | Type::Filesize, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "greater-than or equal comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "greater-than or equal comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::Equal) => match (&lhs.ty, &rhs.ty) {
                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                _ => (Type::Bool, None),
            },
            Operator::Comparison(Comparison::NotEqual) => match (&lhs.ty, &rhs.ty) {
                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                _ => (Type::Bool, None),
            },
            Operator::Comparison(Comparison::RegexMatch) => match (&lhs.ty, &rhs.ty) {
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::String, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "regex matching".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "regex matching".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::NotRegexMatch) => match (&lhs.ty, &rhs.ty) {
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::String, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "regex matching".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "regex matching".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::StartsWith) => match (&lhs.ty, &rhs.ty) {
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::String, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "starts-with comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "starts-with comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::EndsWith) => match (&lhs.ty, &rhs.ty) {
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::String, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "ends-with comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "ends-with comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::In) => match (&lhs.ty, &rhs.ty) {
                (t, Type::List(u)) if type_compatible(t, u) => (Type::Bool, None),
                (Type::Int | Type::Float | Type::Number, Type::Range) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::String, Type::Record(_)) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),
                (Type::Int | Type::Float | Type::String, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "subset comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "subset comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Comparison(Comparison::NotIn) => match (&lhs.ty, &rhs.ty) {
                (t, Type::List(u)) if type_compatible(t, u) => (Type::Bool, None),
                (Type::Int | Type::Float | Type::Number, Type::Range) => (Type::Bool, None),
                (Type::String, Type::String) => (Type::Bool, None),
                (Type::String, Type::Record(_)) => (Type::Bool, None),

                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),

                (Type::Any, _) => (Type::Bool, None),
                (_, Type::Any) => (Type::Bool, None),
                (Type::Int | Type::Float | Type::String, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "subset comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "subset comparison".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Bits(Bits::ShiftLeft)
            | Operator::Bits(Bits::ShiftRight)
            | Operator::Bits(Bits::BitOr)
            | Operator::Bits(Bits::BitXor)
            | Operator::Bits(Bits::BitAnd) => match (&lhs.ty, &rhs.ty) {
                (Type::Int, Type::Int) => (Type::Int, None),

                (Type::Any, _) => (Type::Any, None),
                (_, Type::Any) => (Type::Any, None),
                (Type::Int, _) => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationRHS(
                            "bit operations".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                            rhs.span,
                            rhs.ty.clone(),
                        )),
                    )
                }
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    (
                        Type::Any,
                        Some(ParseError::UnsupportedOperationLHS(
                            "bit operations".into(),
                            op.span,
                            lhs.span,
                            lhs.ty.clone(),
                        )),
                    )
                }
            },
            Operator::Assignment(Assignment::ConcatAssign) => {
                check_concat(working_set, lhs, rhs, op)
            }
            Operator::Assignment(_) => match (&lhs.ty, &rhs.ty) {
                (x, y) if x == y => (Type::Nothing, None),
                (Type::Any, _) => (Type::Nothing, None),
                (_, Type::Any) => (Type::Nothing, None),
                (Type::List(_), Type::List(_)) => (Type::Nothing, None),
                (x, y) => (
                    Type::Nothing,
                    Some(ParseError::Mismatch(x.to_string(), y.to_string(), rhs.span)),
                ),
            },
        },
        _ => {
            *op = Expression::garbage(working_set, op.span);

            (
                Type::Any,
                Some(ParseError::IncompleteMathExpression(op.span)),
            )
        }
    }
}

pub fn check_pipeline_type(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
    input_type: Type,
) -> (Type, Option<Vec<ParseError>>) {
    let mut current_type = input_type;

    let mut output_errors: Option<Vec<ParseError>> = None;

    'elem: for elem in &pipeline.elements {
        if elem.redirection.is_some() {
            current_type = Type::Any;
        } else if let Expr::Call(call) = &elem.expr.expr {
            let decl = working_set.get_decl(call.decl_id);

            if current_type == Type::Any {
                let mut new_current_type = None;
                for (_, call_output) in decl.signature().input_output_types {
                    if let Some(inner_current_type) = &new_current_type {
                        if inner_current_type == &Type::Any {
                            break;
                        } else if inner_current_type != &call_output {
                            // Union unequal types to Any for now
                            new_current_type = Some(Type::Any)
                        }
                    } else {
                        new_current_type = Some(call_output.clone())
                    }
                }

                if let Some(new_current_type) = new_current_type {
                    current_type = new_current_type
                } else {
                    current_type = Type::Any;
                }
                continue 'elem;
            } else {
                for (call_input, call_output) in decl.signature().input_output_types {
                    if type_compatible(&call_input, &current_type) {
                        current_type = call_output.clone();
                        continue 'elem;
                    }
                }
            }

            if !decl.signature().input_output_types.is_empty() {
                if let Some(output_errors) = &mut output_errors {
                    output_errors.push(ParseError::InputMismatch(current_type, call.head))
                } else {
                    output_errors = Some(vec![ParseError::InputMismatch(current_type, call.head)]);
                }
            }
            current_type = Type::Any;
        } else {
            current_type = elem.expr.ty.clone();
        }
    }

    (current_type, output_errors)
}

pub fn check_block_input_output(working_set: &StateWorkingSet, block: &Block) -> Vec<ParseError> {
    // let inputs = block.input_types();
    let mut output_errors = vec![];

    for (input_type, output_type) in &block.signature.input_output_types {
        let mut current_type = input_type.clone();
        let mut current_output_type = Type::Nothing;

        for pipeline in &block.pipelines {
            let (checked_output_type, err) =
                check_pipeline_type(working_set, pipeline, current_type);
            current_output_type = checked_output_type;
            current_type = Type::Nothing;
            if let Some(err) = err {
                output_errors.extend_from_slice(&err);
            }
        }

        if !type_compatible(output_type, &current_output_type)
            && output_type != &Type::Any
            && current_output_type != Type::Any
        {
            let span = if block.pipelines.is_empty() {
                if let Some(span) = block.span {
                    span
                } else {
                    continue;
                }
            } else {
                block
                    .pipelines
                    .last()
                    .expect("internal error: we should have pipelines")
                    .elements
                    .last()
                    .expect("internal error: we should have elements")
                    .expr
                    .span
            };

            output_errors.push(ParseError::OutputMismatch(
                output_type.clone(),
                current_output_type.clone(),
                span,
            ))
        }
    }

    if block.signature.input_output_types.is_empty() {
        let mut current_type = Type::Any;

        for pipeline in &block.pipelines {
            let (_, err) = check_pipeline_type(working_set, pipeline, current_type);
            current_type = Type::Nothing;

            if let Some(err) = err {
                output_errors.extend_from_slice(&err);
            }
        }
    }

    output_errors
}

fn check_concat(
    working_set: &mut StateWorkingSet,
    lhs: &Expression,
    rhs: &Expression,
    op: &mut Expression,
) -> (Type, Option<ParseError>) {
    match (&lhs.ty, &rhs.ty) {
        (Type::List(a), Type::List(b)) => {
            if a == b {
                (Type::List(a.clone()), None)
            } else {
                (Type::List(Box::new(Type::Any)), None)
            }
        }
        (Type::Table(a), Type::Table(_)) => (Type::Table(a.clone()), None),
        (Type::String, Type::String) => (Type::String, None),
        (Type::Binary, Type::Binary) => (Type::Binary, None),
        (Type::Any, _) | (_, Type::Any) => (Type::Any, None),
        (Type::Table(_) | Type::List(_) | Type::String | Type::Binary, _)
        | (_, Type::Table(_) | Type::List(_) | Type::String | Type::Binary) => {
            *op = Expression::garbage(working_set, op.span);
            (
                Type::Any,
                Some(ParseError::UnsupportedOperationRHS(
                    "concatenation".into(),
                    op.span,
                    lhs.span,
                    lhs.ty.clone(),
                    rhs.span,
                    rhs.ty.clone(),
                )),
            )
        }
        _ => {
            *op = Expression::garbage(working_set, op.span);
            (
                Type::Any,
                Some(ParseError::UnsupportedOperationLHS(
                    "concatenation".into(),
                    op.span,
                    lhs.span,
                    lhs.ty.clone(),
                )),
            )
        }
    }
}

/// If one of the parts of the range isn't a number, a parse error is added to the working set
pub fn check_range_types(working_set: &mut StateWorkingSet, range: &mut Range) {
    let next_op_span = if range.next.is_some() {
        range.operator.next_op_span
    } else {
        range.operator.span
    };
    match (&mut range.from, &mut range.next, &mut range.to) {
        (Some(expr), _, _) | (None, Some(expr), Some(_)) | (None, None, Some(expr))
            if !type_compatible(&Type::Number, &expr.ty) =>
        {
            working_set.error(ParseError::UnsupportedOperationLHS(
                String::from("range"),
                next_op_span,
                expr.span,
                expr.ty.clone(),
            ));
            *expr = Expression::garbage(working_set, expr.span);
        }
        (Some(lhs), Some(rhs), _) if !type_compatible(&Type::Number, &rhs.ty) => {
            working_set.error(ParseError::UnsupportedOperationRHS(
                String::from("range"),
                next_op_span,
                lhs.span,
                lhs.ty.clone(),
                rhs.span,
                rhs.ty.clone(),
            ));
            *rhs = Expression::garbage(working_set, rhs.span);
        }
        (Some(lhs), Some(rhs), _) | (Some(lhs), None, Some(rhs)) | (None, Some(lhs), Some(rhs))
            if !type_compatible(&Type::Number, &rhs.ty) =>
        {
            working_set.error(ParseError::UnsupportedOperationRHS(
                String::from("range"),
                range.operator.span,
                lhs.span,
                lhs.ty.clone(),
                rhs.span,
                rhs.ty.clone(),
            ));
            *rhs = Expression::garbage(working_set, rhs.span);
        }
        (Some(from), Some(next), Some(to)) if !type_compatible(&Type::Number, &to.ty) => {
            working_set.error(ParseError::UnsupportedOperationTernary(
                String::from("range"),
                range.operator.span,
                from.span,
                from.ty.clone(),
                next.span,
                next.ty.clone(),
                to.span,
                to.ty.clone(),
            ));
            *to = Expression::garbage(working_set, to.span);
        }
        _ => (),
    }
}
