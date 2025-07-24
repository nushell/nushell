use nu_protocol::{
    ParseError, Span, Type,
    ast::{Assignment, Block, Comparison, Expr, Expression, Math, Operator, Pipeline, Range},
    engine::StateWorkingSet,
};

fn type_error(
    op: Operator,
    op_span: Span,
    lhs: &Expression,
    rhs: &Expression,
    is_supported: fn(&Type) -> bool,
) -> (Type, Option<ParseError>) {
    let is_supported = |ty| is_supported(ty) || matches!(ty, Type::Any | Type::Custom(_));
    let err = match (is_supported(&lhs.ty), is_supported(&rhs.ty)) {
        (true, true) => ParseError::OperatorIncompatibleTypes {
            op: op.as_str(),
            lhs: lhs.ty.clone(),
            rhs: rhs.ty.clone(),
            op_span,
            lhs_span: lhs.span,
            rhs_span: rhs.span,
            help: None,
        },
        (true, false) => ParseError::OperatorUnsupportedType {
            op: op.as_str(),
            unsupported: rhs.ty.clone(),
            op_span,
            unsupported_span: rhs.span,
            help: None,
        },
        (false, _) => ParseError::OperatorUnsupportedType {
            op: op.as_str(),
            unsupported: lhs.ty.clone(),
            op_span,
            unsupported_span: lhs.span,
            help: None,
        },
    };
    (Type::Any, Some(err))
}

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

// TODO: rework type checking for Custom values
pub fn math_result_type(
    working_set: &mut StateWorkingSet,
    lhs: &mut Expression,
    op: &mut Expression,
    rhs: &mut Expression,
) -> (Type, Option<ParseError>) {
    let &Expr::Operator(operator) = &op.expr else {
        *op = Expression::garbage(working_set, op.span);
        return (
            Type::Any,
            Some(ParseError::IncompleteMathExpression(op.span)),
        );
    };
    match operator {
        Operator::Math(Math::Add) => match (&lhs.ty, &rhs.ty) {
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
            // TODO: should this include glob
            (Type::Date, Type::Duration) => (Type::Date, None),
            (Type::Duration, Type::Date) => (Type::Date, None),
            (Type::Duration, Type::Duration) => (Type::Duration, None),
            (Type::Filesize, Type::Filesize) => (Type::Filesize, None),
            (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
            (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
            (Type::Any, _) => (Type::Any, None),
            (_, Type::Any) => (Type::Any, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int
                            | Type::Float
                            | Type::Number
                            | Type::String
                            | Type::Date
                            | Type::Duration
                            | Type::Filesize,
                    )
                })
            }
        },
        Operator::Math(Math::Subtract) => match (&lhs.ty, &rhs.ty) {
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
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int
                            | Type::Float
                            | Type::Number
                            | Type::Date
                            | Type::Duration
                            | Type::Filesize
                    )
                })
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
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Duration | Type::Filesize,
                    )
                })
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
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Filesize | Type::Duration
                    )
                })
            }
        },
        Operator::Math(Math::FloorDivide) => match (&lhs.ty, &rhs.ty) {
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
            (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
            (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
            (Type::Any, _) => (Type::Any, None),
            (_, Type::Any) => (Type::Any, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Filesize | Type::Duration
                    )
                })
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
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Filesize | Type::Duration
                    )
                })
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
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(ty, Type::Int | Type::Float | Type::Number)
                })
            }
        },
        Operator::Math(Math::Concatenate) => match (&lhs.ty, &rhs.ty) {
            (Type::List(a), Type::List(b)) => {
                if a == b {
                    (Type::list(a.as_ref().clone()), None)
                } else {
                    (Type::list(Type::Any), None)
                }
            }
            (Type::Table(a), Type::Table(_)) => (Type::Table(a.clone()), None),
            (Type::Table(table), Type::List(list)) => {
                if matches!(list.as_ref(), Type::Record(..)) {
                    (Type::Table(table.clone()), None)
                } else {
                    (Type::list(Type::Any), None)
                }
            }
            (Type::List(list), Type::Table(_)) => {
                if matches!(list.as_ref(), Type::Record(..)) {
                    (Type::list(list.as_ref().clone()), None)
                } else {
                    (Type::list(Type::Any), None)
                }
            }
            (Type::String, Type::String) => (Type::String, None),
            // TODO: should this include glob
            (Type::Binary, Type::Binary) => (Type::Binary, None),
            (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
            (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
            (Type::Any, _) | (_, Type::Any) => (Type::Any, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                let is_supported = |ty: &Type| {
                    matches!(
                        ty,
                        Type::List(_)
                            | Type::Table(_)
                            | Type::String
                            | Type::Binary
                            | Type::Any
                            | Type::Custom(_)
                    )
                };
                let help = if matches!(lhs.ty, Type::List(_) | Type::Table(_))
                    || matches!(rhs.ty, Type::List(_) | Type::Table(_))
                {
                    Some(
                        "if you meant to append a value to a list or a record to a table, use the `append` command or wrap the value in a list. For example: `$list ++ $value` should be `$list ++ [$value]` or `$list | append $value`.",
                    )
                } else {
                    None
                };
                let err = match (is_supported(&lhs.ty), is_supported(&rhs.ty)) {
                    (true, true) => ParseError::OperatorIncompatibleTypes {
                        op: operator.as_str(),
                        lhs: lhs.ty.clone(),
                        rhs: rhs.ty.clone(),
                        op_span: op.span,
                        lhs_span: lhs.span,
                        rhs_span: rhs.span,
                        help,
                    },
                    (true, false) => ParseError::OperatorUnsupportedType {
                        op: operator.as_str(),
                        unsupported: rhs.ty.clone(),
                        op_span: op.span,
                        unsupported_span: rhs.span,
                        help,
                    },
                    (false, _) => ParseError::OperatorUnsupportedType {
                        op: operator.as_str(),
                        unsupported: lhs.ty.clone(),
                        op_span: op.span,
                        unsupported_span: lhs.span,
                        help,
                    },
                };
                (Type::Any, Some(err))
            }
        },
        Operator::Boolean(_) => match (&lhs.ty, &rhs.ty) {
            (Type::Bool, Type::Bool) => (Type::Bool, None),
            (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
            (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
            (Type::Any, _) => (Type::Any, None),
            (_, Type::Any) => (Type::Any, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::Bool))
            }
        },
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
            (Type::Nothing, _) => (Type::Nothing, None), // TODO: is this right
            (_, Type::Nothing) => (Type::Nothing, None), // TODO: is this right
            // TODO: should this include:
            // - binary
            // - glob
            // - list
            // - table
            // - record
            // - range
            (Type::Any, _) => (Type::Bool, None),
            (_, Type::Any) => (Type::Bool, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int
                            | Type::Float
                            | Type::Number
                            | Type::String
                            | Type::Filesize
                            | Type::Duration
                            | Type::Date
                            | Type::Bool
                            | Type::Nothing
                    )
                })
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
            (Type::Nothing, _) => (Type::Nothing, None), // TODO: is this right
            (_, Type::Nothing) => (Type::Nothing, None), // TODO: is this right
            // TODO: should this include:
            // - binary
            // - glob
            // - list
            // - table
            // - record
            // - range
            (Type::Any, _) => (Type::Bool, None),
            (_, Type::Any) => (Type::Bool, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int
                            | Type::Float
                            | Type::Number
                            | Type::String
                            | Type::Filesize
                            | Type::Duration
                            | Type::Date
                            | Type::Bool
                            | Type::Nothing
                    )
                })
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
            (Type::Nothing, _) => (Type::Nothing, None), // TODO: is this right
            (_, Type::Nothing) => (Type::Nothing, None), // TODO: is this right
            // TODO: should this include:
            // - binary
            // - glob
            // - list
            // - table
            // - record
            // - range
            (Type::Any, _) => (Type::Bool, None),
            (_, Type::Any) => (Type::Bool, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int
                            | Type::Float
                            | Type::Number
                            | Type::String
                            | Type::Filesize
                            | Type::Duration
                            | Type::Date
                            | Type::Bool
                            | Type::Nothing
                    )
                })
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
            (Type::Nothing, _) => (Type::Nothing, None), // TODO: is this right
            (_, Type::Nothing) => (Type::Nothing, None), // TODO: is this right
            // TODO: should this include:
            // - binary
            // - glob
            // - list
            // - table
            // - record
            // - range
            (Type::Any, _) => (Type::Bool, None),
            (_, Type::Any) => (Type::Bool, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int
                            | Type::Float
                            | Type::Number
                            | Type::String
                            | Type::Filesize
                            | Type::Duration
                            | Type::Date
                            | Type::Bool
                            | Type::Nothing
                    )
                })
            }
        },
        Operator::Comparison(Comparison::Equal | Comparison::NotEqual) => {
            match (&lhs.ty, &rhs.ty) {
                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
                _ => (Type::Bool, None),
            }
        }
        Operator::Comparison(Comparison::RegexMatch | Comparison::NotRegexMatch) => {
            match (&lhs.ty, &rhs.ty) {
                (Type::String | Type::Any, Type::String | Type::Any) => (Type::Bool, None),
                // TODO: should this include glob?
                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::String))
                }
            }
        }
        Operator::Comparison(Comparison::StartsWith | Comparison::EndsWith) => {
            match (&lhs.ty, &rhs.ty) {
                (Type::String | Type::Any, Type::String | Type::Any) => (Type::Bool, None),
                // TODO: should this include glob?
                (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
                (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
                _ => {
                    *op = Expression::garbage(working_set, op.span);
                    type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::String))
                }
            }
        }
        Operator::Comparison(Comparison::In | Comparison::NotIn) => match (&lhs.ty, &rhs.ty) {
            (t, Type::List(u)) if type_compatible(t, u) => (Type::Bool, None),
            (Type::Int | Type::Float | Type::Number, Type::Range) => (Type::Bool, None),
            (Type::String, Type::String) => (Type::Bool, None),
            (Type::String, Type::Record(_)) => (Type::Bool, None),
            (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
            (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
            (Type::Any, _) => (Type::Bool, None),
            (_, Type::Any) => (Type::Bool, None),
            _ => {
                let err = if matches!(
                    &rhs.ty,
                    Type::List(_)
                        | Type::Range
                        | Type::String
                        | Type::Record(_)
                        | Type::Custom(_)
                        | Type::Any
                ) {
                    ParseError::OperatorIncompatibleTypes {
                        op: operator.as_str(),
                        lhs: lhs.ty.clone(),
                        rhs: rhs.ty.clone(),
                        op_span: op.span,
                        lhs_span: lhs.span,
                        rhs_span: rhs.span,
                        help: None,
                    }
                } else {
                    ParseError::OperatorUnsupportedType {
                        op: operator.as_str(),
                        unsupported: rhs.ty.clone(),
                        op_span: op.span,
                        unsupported_span: rhs.span,
                        help: None,
                    }
                };
                *op = Expression::garbage(working_set, op.span);
                (Type::Any, Some(err))
            }
        },
        Operator::Comparison(Comparison::Has | Comparison::NotHas) => match (&lhs.ty, &rhs.ty) {
            (Type::List(u), t) if type_compatible(u, t) => (Type::Bool, None),
            (Type::Range, Type::Int | Type::Float | Type::Number) => (Type::Bool, None),
            (Type::String, Type::String) => (Type::Bool, None),
            (Type::Record(_), Type::String) => (Type::Bool, None),
            (Type::Custom(a), Type::Custom(b)) if a == b => (Type::Custom(a.clone()), None),
            (Type::Custom(a), _) => (Type::Custom(a.clone()), None),
            (Type::Any, _) => (Type::Bool, None),
            (_, Type::Any) => (Type::Bool, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                let err = if matches!(
                    &lhs.ty,
                    Type::List(_)
                        | Type::Range
                        | Type::String
                        | Type::Record(_)
                        | Type::Custom(_)
                        | Type::Any
                ) {
                    ParseError::OperatorIncompatibleTypes {
                        op: operator.as_str(),
                        lhs: lhs.ty.clone(),
                        rhs: rhs.ty.clone(),
                        op_span: op.span,
                        lhs_span: lhs.span,
                        rhs_span: rhs.span,
                        help: None,
                    }
                } else {
                    ParseError::OperatorUnsupportedType {
                        op: operator.as_str(),
                        unsupported: lhs.ty.clone(),
                        op_span: op.span,
                        unsupported_span: lhs.span,
                        help: None,
                    }
                };
                (Type::Any, Some(err))
            }
        },
        Operator::Bits(_) => match (&lhs.ty, &rhs.ty) {
            (Type::Int, Type::Int) => (Type::Int, None),
            (Type::Any, _) => (Type::Any, None),
            (_, Type::Any) => (Type::Any, None),
            _ => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::Int))
            }
        },
        Operator::Assignment(Assignment::AddAssign) => {
            compound_assignment_result_type(working_set, lhs, op, rhs, operator, Math::Add)
        }
        Operator::Assignment(Assignment::ConcatenateAssign) => {
            compound_assignment_result_type(working_set, lhs, op, rhs, operator, Math::Concatenate)
        }
        Operator::Assignment(Assignment::DivideAssign) => {
            compound_assignment_result_type(working_set, lhs, op, rhs, operator, Math::Divide)
        }
        Operator::Assignment(Assignment::MultiplyAssign) => {
            compound_assignment_result_type(working_set, lhs, op, rhs, operator, Math::Multiply)
        }
        Operator::Assignment(Assignment::SubtractAssign) => {
            compound_assignment_result_type(working_set, lhs, op, rhs, operator, Math::Subtract)
        }
        Operator::Assignment(Assignment::Assign) => {
            let err = if type_compatible(&lhs.ty, &rhs.ty) {
                None
            } else {
                *op = Expression::garbage(working_set, op.span);
                Some(ParseError::OperatorIncompatibleTypes {
                    op: operator.as_str(),
                    lhs: lhs.ty.clone(),
                    rhs: rhs.ty.clone(),
                    op_span: op.span,
                    lhs_span: lhs.span,
                    rhs_span: rhs.span,
                    help: None,
                })
            };
            (Type::Nothing, err)
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
            working_set.error(ParseError::OperatorUnsupportedType {
                op: "..",
                unsupported: expr.ty.clone(),
                op_span: next_op_span,
                unsupported_span: expr.span,
                help: None,
            });
            *expr = Expression::garbage(working_set, expr.span);
        }
        (Some(_), Some(rhs), _) | (Some(_), None, Some(rhs)) | (None, Some(_), Some(rhs))
            if !type_compatible(&Type::Number, &rhs.ty) =>
        {
            working_set.error(ParseError::OperatorUnsupportedType {
                op: "..",
                unsupported: rhs.ty.clone(),
                op_span: next_op_span,
                unsupported_span: rhs.span,
                help: None,
            });
            *rhs = Expression::garbage(working_set, rhs.span);
        }
        (Some(_), Some(_), Some(to)) if !type_compatible(&Type::Number, &to.ty) => {
            working_set.error(ParseError::OperatorUnsupportedType {
                op: "..",
                unsupported: to.ty.clone(),
                op_span: next_op_span,
                unsupported_span: to.span,
                help: None,
            });
            *to = Expression::garbage(working_set, to.span);
        }
        _ => (),
    }
}

/// Get the result type for a compound assignment operator
fn compound_assignment_result_type(
    working_set: &mut StateWorkingSet,
    lhs: &mut Expression,
    op: &mut Expression,
    rhs: &mut Expression,
    operator: Operator,
    operation: Math,
) -> (Type, Option<ParseError>) {
    let math_expr = Expr::Operator(Operator::Math(operation));
    let mut math_op = Expression::new(working_set, math_expr, op.span, Type::Any);
    match math_result_type(working_set, lhs, &mut math_op, rhs) {
        // There was a type error in the math expression, so propagate it
        (_, Some(err)) => (Type::Any, Some(err)),
        // Operation type check okay, check regular assignment
        (ty, None) if type_compatible(&lhs.ty, &ty) => (Type::Nothing, None),
        // The math expression is fine, but we can't store the result back into the variable due to type mismatch
        (_, None) => {
            *op = Expression::garbage(working_set, op.span);
            let err = ParseError::OperatorIncompatibleTypes {
                op: operator.as_str(),
                lhs: lhs.ty.clone(),
                rhs: rhs.ty.clone(),
                op_span: op.span,
                lhs_span: lhs.span,
                rhs_span: rhs.span,
                help: Some(
                    "The result type of this operation is not compatible with the type of the variable.",
                ),
            };
            (Type::Nothing, Some(err))
        }
    }
}
