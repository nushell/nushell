use crate::{parser::Operator, parser_state::Type, Expr, Expression, ParseError, ParserWorkingSet};

impl<'a> ParserWorkingSet<'a> {
    pub fn type_compatible(lhs: &Type, rhs: &Type) -> bool {
        match (lhs, rhs) {
            (Type::List(c), Type::List(d)) => ParserWorkingSet::type_compatible(c, d),
            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true,
            (lhs, rhs) => lhs == rhs,
        }
    }

    pub fn math_result_type(
        &self,
        lhs: &mut Expression,
        op: &mut Expression,
        rhs: &mut Expression,
    ) -> (Type, Option<ParseError>) {
        match &op.expr {
            Expr::Operator(operator) => match operator {
                Operator::Equal => (Type::Bool, None),
                Operator::Multiply => match (&lhs.ty, &rhs.ty) {
                    (Type::Int, Type::Int) => (Type::Int, None),
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
                Operator::Plus => match (&lhs.ty, &rhs.ty) {
                    (Type::Int, Type::Int) => (Type::Int, None),
                    (Type::String, Type::String) => (Type::String, None),
                    (Type::Unknown, _) => (Type::Unknown, None),
                    (_, Type::Unknown) => (Type::Unknown, None),
                    (Type::Int, _) => {
                        *rhs = Expression::garbage(rhs.span);
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
}
