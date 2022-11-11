use std::ops::{Add, Div, Mul, Rem, Sub};

use super::NuExpression;
use nu_protocol::{
    ast::{Comparison, Math, Operator},
    CustomValue, ShellError, Span, Type, Value,
};
use polars::prelude::Expr;

// CustomValue implementation for NuDataFrame
impl CustomValue for NuExpression {
    fn typetag_name(&self) -> &'static str {
        "expression"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = NuExpression(self.0.clone());

        Value::CustomValue {
            val: Box::new(cloned),
            span,
        }
    }

    fn value_string(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(self.to_value(span))
    }

    fn to_json(&self) -> nu_json::Value {
        nu_json::Value::Null
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        compute_with_value(self, lhs_span, operator, op, right)
    }
}

fn compute_with_value(
    left: &NuExpression,
    lhs_span: Span,
    operator: Operator,
    op: Span,
    right: &Value,
) -> Result<Value, ShellError> {
    match right {
        Value::CustomValue {
            val: rhs,
            span: rhs_span,
        } => {
            let rhs = rhs.as_any().downcast_ref::<NuExpression>().ok_or_else(|| {
                ShellError::DowncastNotPossible(
                    "Unable to create expression".to_string(),
                    *rhs_span,
                )
            })?;

            match rhs.as_ref() {
                polars::prelude::Expr::Literal(..) => {
                    with_operator(operator, left, rhs, lhs_span, right.span()?, op)
                }
                _ => Err(ShellError::TypeMismatch(
                    "Only literal expressions or number".into(),
                    right.span()?,
                )),
            }
        }
        _ => {
            let rhs = NuExpression::try_from_value(right.clone())?;
            with_operator(operator, left, &rhs, lhs_span, right.span()?, op)
        }
    }
}

fn with_operator(
    operator: Operator,
    left: &NuExpression,
    right: &NuExpression,
    lhs_span: Span,
    rhs_span: Span,
    op_span: Span,
) -> Result<Value, ShellError> {
    match operator {
        Operator::Math(Math::Plus) => apply_arithmetic(left, right, lhs_span, Add::add),
        Operator::Math(Math::Minus) => apply_arithmetic(left, right, lhs_span, Sub::sub),
        Operator::Math(Math::Multiply) => apply_arithmetic(left, right, lhs_span, Mul::mul),
        Operator::Math(Math::Divide) => apply_arithmetic(left, right, lhs_span, Div::div),
        Operator::Math(Math::Modulo) => apply_arithmetic(left, right, lhs_span, Rem::rem),
        Operator::Math(Math::FloorDivision) => apply_arithmetic(left, right, lhs_span, Div::div),
        Operator::Comparison(Comparison::Equal) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::eq)
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::NotEqual) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::neq)
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::GreaterThan) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::gt)
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::GreaterThanOrEqual) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::gt_eq)
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::LessThan) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::lt)
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::LessThanOrEqual) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::lt_eq)
            .into_value(lhs_span)),
        _ => Err(ShellError::OperatorMismatch {
            op_span,
            lhs_ty: Type::Custom(left.typetag_name().into()),
            lhs_span,
            rhs_ty: Type::Custom(right.typetag_name().into()),
            rhs_span,
        }),
    }
}

fn apply_arithmetic<F>(
    left: &NuExpression,
    right: &NuExpression,
    span: Span,
    f: F,
) -> Result<Value, ShellError>
where
    F: Fn(Expr, Expr) -> Expr,
{
    let expr: NuExpression = f(left.as_ref().clone(), right.as_ref().clone()).into();

    Ok(expr.into_value(span))
}
