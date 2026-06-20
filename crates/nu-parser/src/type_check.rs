use nu_protocol::{
    CompareTypes, ParseError, Span, Type, TypeSet,
    ast::{Assignment, Block, Comparison, Expr, Expression, Math, Operator, Pipeline, Range},
    combined_type_string,
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

pub fn type_compatible(dst: &Type, src: &Type) -> bool {
    src.is_assignable_to(dst)
}

fn type_combinations<'a, F, IL, IR>(op: F, lhs_tys: IL, rhs_ty: IR) -> Option<Type>
where
    F: Fn(&Type, &Type) -> Option<Type>,
    IL: IntoIterator<Item = &'a Type>,
    IR: IntoIterator<Item = &'a Type> + Clone,
{
    lhs_tys
        .into_iter()
        .flat_map(|lhs| rhs_ty.clone().into_iter().filter_map(|rhs| op(lhs, rhs)))
        .reduce(Type::union)
}

fn operator_check<F>(op: F, on_any: impl Into<Option<Type>>, lhs: &Type, rhs: &Type) -> Option<Type>
where
    F: Fn(&Type, &Type) -> Option<Type>,
{
    use std::slice::from_ref as as_slice;
    let on_any = on_any.into();

    match (lhs, rhs) {
        (Type::Any, _) | (_, Type::Any) if let Some(on_any) = on_any => Some(on_any),
        (Type::OneOf(lhs_oneof), Type::OneOf(rhs_oneof)) => {
            type_combinations(op, lhs_oneof.iter(), rhs_oneof.iter())
        }
        (Type::OneOf(lhs_oneof), rhs) => type_combinations(op, lhs_oneof.iter(), as_slice(rhs)),
        (lhs, Type::OneOf(rhs_oneof)) => type_combinations(op, as_slice(lhs), rhs_oneof.iter()),
        (lhs, rhs) => op(lhs, rhs),
    }
}

// covers number types for operand type checking. this is correct for (at least) these operations:
// - add
// - subtract
// - multiply
// - floor_divide
// - modulo
// - pow
fn number_types(lhs: &Type, rhs: &Type) -> Option<Type> {
    Some(match (lhs, rhs) {
        (Type::Int, Type::Int) => Type::Int,
        (Type::Int | Type::Number, Type::Int | Type::Number) => Type::Number,
        (Type::Float, Type::Int | Type::Float | Type::Number)
        | (Type::Int | Type::Number, Type::Float) => Type::Float,
        _ => return None,
    })
}

mod math {
    use super::*;

    fn commutative<F>(op: F) -> impl Fn(&Type, &Type) -> Option<Type>
    where
        F: Fn(&Type, &Type) -> Option<Type>,
    {
        move |lhs, rhs| op(lhs, rhs).or_else(|| op(rhs, lhs))
    }

    pub(super) fn add(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn reversible_op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (lhs, rhs) if let Some(out) = number_types(lhs, rhs) => out,

                (Type::String, Type::String) => Type::String,
                // TODO: should this include glob
                (Type::Date, Type::Duration) => Type::Date,

                (Type::Duration, Type::Duration) => Type::Duration,
                (Type::Filesize, Type::Filesize) => Type::Filesize,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(commutative(reversible_op), Type::Any, lhs, rhs)
    }

    pub(super) fn subtract(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (lhs, rhs) if let Some(out) = number_types(lhs, rhs) => out,

                (Type::Date, Type::Date) => Type::Duration,
                (Type::Date, Type::Duration) => Type::Date,

                (Type::Duration, Type::Duration) => Type::Duration,
                (Type::Filesize, Type::Filesize) => Type::Filesize,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(op, Type::Any, lhs, rhs)
    }

    pub(super) fn multiply(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn reversible_op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (lhs, rhs) if let Some(out) = number_types(lhs, rhs) => out,

                (Type::Filesize, Type::Int | Type::Float | Type::Number) => Type::Filesize,
                (Type::Duration, Type::Int | Type::Float | Type::Number) => Type::Duration,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(commutative(reversible_op), Type::Any, lhs, rhs)
    }

    pub(super) fn divide(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (
                    Type::Int | Type::Float | Type::Number,
                    Type::Int | Type::Float | Type::Number,
                ) => Type::Float,

                (Type::Filesize, Type::Filesize) => Type::Float,
                (Type::Duration, Type::Duration) => Type::Float,

                (Type::Filesize, Type::Int | Type::Float | Type::Number) => Type::Filesize,
                (Type::Duration, Type::Int | Type::Float | Type::Number) => Type::Duration,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(op, Type::Any, lhs, rhs)
    }

    pub(super) fn floor_divide(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (lhs, rhs) if let Some(out) = number_types(lhs, rhs) => out,

                (Type::Filesize, Type::Filesize) => Type::Int,
                (Type::Duration, Type::Duration) => Type::Int,

                (Type::Filesize, Type::Int | Type::Float | Type::Number) => Type::Filesize,
                (Type::Duration, Type::Int | Type::Float | Type::Number) => Type::Duration,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(op, Type::Any, lhs, rhs)
    }

    pub(super) fn modulo(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (lhs, rhs) if let Some(out) = number_types(lhs, rhs) => out,

                (Type::Filesize, Type::Filesize) => Type::Filesize,
                (Type::Duration, Type::Duration) => Type::Duration,

                (Type::Filesize, Type::Int | Type::Float | Type::Number) => Type::Filesize,
                (Type::Duration, Type::Int | Type::Float | Type::Number) => Type::Duration,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(op, Type::Any, lhs, rhs)
    }

    pub(super) fn pow(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (lhs, rhs) if let Some(out) = number_types(lhs, rhs) => out,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(op, Type::Any, lhs, rhs)
    }

    pub(super) fn concatenate(lhs: &Type, rhs: &Type) -> Option<Type> {
        fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
            Some(match (lhs, rhs) {
                (Type::List(a), Type::List(b)) => {
                    Type::list(a.as_ref().clone().union(b.as_ref().clone()))
                }
                (Type::Table(a), Type::Table(b)) => Type::Table(a.clone().union(b.clone())),
                (Type::Table(table), Type::List(list)) => {
                    Type::list(Type::Record(table.clone()).union(list.as_ref().clone()))
                }
                (Type::String, Type::String) => Type::String,
                // TODO: should this include glob
                (Type::Binary, Type::Binary) => Type::Binary,

                (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                (Type::Custom(a), _) => Type::Custom(a.clone()),

                _ => return None,
            })
        }
        operator_check(commutative(op), Type::Any, lhs, rhs)
    }
}

fn ord_cmp_op(lhs: &Type, rhs: &Type) -> Option<Type> {
    fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
        Some(match (lhs, rhs) {
            (Type::Int | Type::Float | Type::Number, Type::Int | Type::Float | Type::Number) => {
                Type::Bool
            }

            (Type::String, Type::String) => Type::Bool,
            (Type::Duration, Type::Duration) => Type::Bool,
            (Type::Date, Type::Date) => Type::Bool,
            (Type::Filesize, Type::Filesize) => Type::Bool,
            (Type::Bool, Type::Bool) => Type::Bool,

            (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
            (Type::Custom(a), _) => Type::Custom(a.clone()),

            (Type::Nothing, _) | (_, Type::Nothing) => Type::Nothing, // TODO: is this right
            // TODO: should this include:
            // - binary
            // - glob
            // - list
            // - table
            // - record
            // - range
            _ => return None,
        })
    }
    operator_check(op, Type::Bool, lhs, rhs)
}

fn str_cmp_op(lhs: &Type, rhs: &Type) -> Option<Type> {
    fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
        Some(match (lhs, rhs) {
            (Type::String | Type::Any, Type::String | Type::Any) => Type::Bool,
            // TODO: should this include glob?
            (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
            (Type::Custom(a), _) => Type::Custom(a.clone()),
            _ => return None,
        })
    }
    operator_check(op, None, lhs, rhs)
}

fn in_op(lhs: &Type, rhs: &Type) -> Option<Type> {
    fn op(lhs: &Type, rhs: &Type) -> Option<Type> {
        Some(match (lhs, rhs) {
            (t, Type::List(u)) if type_compatible(t, u) => Type::Bool,
            (Type::Int | Type::Float | Type::Number, Type::Range) => Type::Bool,
            (Type::String, Type::String) => Type::Bool,
            (Type::String, Type::Record(_)) => Type::Bool,
            (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
            (Type::Custom(a), _) => Type::Custom(a.clone()),
            _ => return None,
        })
    }
    operator_check(op, Type::Any, lhs, rhs)
}

fn has_op(lhs: &Type, rhs: &Type) -> Option<Type> {
    in_op(rhs, lhs)
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
        Operator::Math(Math::Add) => match math::add(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
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
        Operator::Math(Math::Subtract) => match math::subtract(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
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
        Operator::Math(Math::Multiply) => match math::multiply(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Duration | Type::Filesize,
                    )
                })
            }
        },
        Operator::Math(Math::Divide) => match math::divide(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Filesize | Type::Duration
                    )
                })
            }
        },
        Operator::Math(Math::FloorDivide) => match math::floor_divide(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Filesize | Type::Duration
                    )
                })
            }
        },
        Operator::Math(Math::Modulo) => match math::modulo(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(
                        ty,
                        Type::Int | Type::Float | Type::Number | Type::Filesize | Type::Duration
                    )
                })
            }
        },
        Operator::Math(Math::Pow) => match math::pow(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| {
                    matches!(ty, Type::Int | Type::Float | Type::Number)
                })
            }
        },
        Operator::Math(Math::Concatenate) => match math::concatenate(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
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
        Operator::Boolean(_) => {
            let out = operator_check(
                |lhs, rhs| {
                    Some(match (lhs, rhs) {
                        (Type::Bool, Type::Bool) => Type::Bool,
                        (Type::Custom(a), Type::Custom(b)) if a == b => Type::Custom(a.clone()),
                        (Type::Custom(a), _) => Type::Custom(a.clone()),
                        _ => return None,
                    })
                },
                Type::Any,
                &lhs.ty,
                &rhs.ty,
            );
            match out {
                Some(ty) => (ty, None),
                None => {
                    *op = Expression::garbage(working_set, op.span);
                    type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::Bool))
                }
            }
        }
        Operator::Comparison(
            Comparison::LessThan
            | Comparison::LessThanOrEqual
            | Comparison::GreaterThan
            | Comparison::GreaterThanOrEqual,
        ) => match ord_cmp_op(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
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
        Operator::Comparison(
            Comparison::RegexMatch
            | Comparison::NotRegexMatch
            | Comparison::StartsWith
            | Comparison::NotStartsWith
            | Comparison::EndsWith
            | Comparison::NotEndsWith,
        ) => match str_cmp_op(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
                *op = Expression::garbage(working_set, op.span);
                type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::String))
            }
        },
        Operator::Comparison(Comparison::In | Comparison::NotIn) => match in_op(&lhs.ty, &rhs.ty) {
            Some(ty) => (ty, None),
            None => {
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
        Operator::Comparison(Comparison::Has | Comparison::NotHas) => {
            match has_op(&lhs.ty, &rhs.ty) {
                Some(ty) => (ty, None),
                None => {
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
            }
        }
        Operator::Bits(_) => {
            let out = operator_check(
                |lhs, rhs| {
                    Some(match (lhs, rhs) {
                        (Type::Int, Type::Int) => Type::Int,
                        _ => return None,
                    })
                },
                Type::Any,
                &lhs.ty,
                &rhs.ty,
            );
            match out {
                Some(ty) => (ty, None),
                None => {
                    *op = Expression::garbage(working_set, op.span);
                    type_error(operator, op.span, lhs, rhs, |ty| matches!(ty, Type::Int))
                }
            }
        }
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

/// Determine the possible output types of a pipeline.
pub fn check_pipeline_type(
    working_set: &StateWorkingSet,
    pipeline: &Pipeline,
    input_type: Type,
) -> (Type, Option<Vec<ParseError>>) {
    let mut input_type = input_type;
    let mut output_errors: Option<Vec<ParseError>> = None;

    for elem in &pipeline.elements {
        if elem.redirection.is_some() {
            input_type = Type::Any;
            continue;
        }
        // Only handle pipeline type checking of calls here.
        let Expr::Call(call) = &elem.expr.expr else {
            // NOTE[1]: Calls with `$in` are wrapped in `Expr::Collect` so are handled in this
            // branch.
            // This allows `ls | sort-by { open -r $in.name | lines | length }` to type
            // check, despite the fact `open` does not support `record` input.
            // This is thanks to `parse_internal_call` adding `Type::Nothing` to possible input
            // types.
            //
            // see NOTE[2]
            input_type = elem.expr.ty.clone();
            continue;
        };
        // Dynamic percent dispatch uses a placeholder decl_id that is rewritten later in IR.
        // Defer type constraints for this call at parse/type-check time.
        if call.parser_info.contains_key("percent_forced_builtin") {
            input_type = Type::Any;
            continue;
        }

        let output_type = working_set
            .get_decl(call.decl_id)
            .signature()
            // NOTE[2]: unlike `parse_internal_call`, `Type::Nothing` is not added to input types.
            .get_output_type(Some(input_type.clone()));

        if let Some(output_type) = output_type {
            input_type = output_type;
            continue;
        }

        let types_string = match &input_type {
            Type::OneOf(types) => combined_type_string(types.iter(), "or"),
            ty => combined_type_string(std::slice::from_ref(ty).iter(), "or"),
        };

        let Some(types_string) = types_string else {
            output_errors
                .get_or_insert_default()
                .push(ParseError::InternalError(
                    "Pipeline has no type at this point".to_string(),
                    elem.expr.span,
                ));
            continue;
        };

        output_errors
            .get_or_insert_default()
            .push(ParseError::InputMismatch(types_string, call.head));
    }

    (input_type, output_errors)
}

pub fn check_block_input_output(working_set: &StateWorkingSet, block: &Block) -> Vec<ParseError> {
    // let inputs = block.input_types();
    let mut output_errors = vec![];

    for (input_type, output_type) in &block.signature.input_output_types {
        let current_output_type = match block.pipelines.as_slice() {
            [] => input_type.clone(),
            pipelines => {
                pipelines
                    .iter()
                    .fold((input_type.clone(), Type::Nothing), |(ct, _), pipeline| {
                        let (checked_output_type, err) =
                            check_pipeline_type(working_set, pipeline, ct);
                        if let Some(err) = err {
                            output_errors.extend(err);
                        }
                        (Type::Nothing, checked_output_type)
                    })
                    .1
            }
        };

        if current_output_type.is_assignable_to(output_type) {
            continue;
        };

        // Error handling
        let span = match block.pipelines.as_slice() {
            [] => match block.span {
                Some(span) => span,
                None => continue,
            },
            [.., last] => {
                last.elements
                    .last()
                    .expect("internal error: we should have elements")
                    .expr
                    .span
            }
        };

        let current_ty_string = match &current_output_type {
            Type::OneOf(types) => combined_type_string(types.iter(), "or"),
            ty => combined_type_string(std::slice::from_ref(ty).iter(), "or"),
        };

        let Some(current_ty_string) = current_ty_string else {
            output_errors.push(ParseError::InternalError(
                "Block has no type at this point".to_string(),
                span,
            ));
            continue;
        };

        output_errors.push(ParseError::OutputMismatch(
            output_type.clone(),
            current_ty_string,
            span,
        ))
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
