use crate::{parser::Operator, parser_state::Type, Expr, Expression, ParseError, ParserWorkingSet};

impl<'a> ParserWorkingSet<'a> {
    pub fn math_result_type(
        &self,
        lhs: &mut Expression,
        op: &mut Expression,
        rhs: &mut Expression,
    ) -> (Type, Option<ParseError>) {
        match &op.expr {
            Expr::Operator(operator) => match operator {
                Operator::Multiply => match (&lhs.ty, &rhs.ty) {
                    (Type::Int, Type::Int) => (Type::Int, None),
                    (Type::Unknown, _) => (Type::Unknown, None),
                    (_, Type::Unknown) => (Type::Unknown, None),
                    _ => {
                        *op = Expression::garbage(op.span);
                        (
                            Type::Unknown,
                            Some(ParseError::Mismatch("math".into(), op.span)),
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
                            Some(ParseError::Mismatch("int".into(), rhs.span)),
                        )
                    }
                    _ => {
                        *op = Expression::garbage(op.span);
                        (
                            Type::Unknown,
                            Some(ParseError::Mismatch("math".into(), op.span)),
                        )
                    }
                },
                _ => {
                    *op = Expression::garbage(op.span);
                    (
                        Type::Unknown,
                        Some(ParseError::Mismatch("math".into(), op.span)),
                    )
                }
            },
            _ => {
                *op = Expression::garbage(op.span);
                (
                    Type::Unknown,
                    Some(ParseError::Mismatch("operator".into(), op.span)),
                )
            }
        }
    }
}
