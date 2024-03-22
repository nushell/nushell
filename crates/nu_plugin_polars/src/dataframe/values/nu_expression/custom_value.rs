use std::ops::{Add, Div, Mul, Rem, Sub};

use crate::DataFrameCache;

use super::NuExpression;
use nu_protocol::{
    ast::{Comparison, Math, Operator},
    CustomValue, ShellError, Span, Type, Value,
};
use polars::prelude::Expr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TYPE_NAME: &str = "NuExpression";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NuExpressionCustomValue {
    pub id: Uuid,
    #[serde(skip)]
    pub expr: Option<NuExpression>,
}

impl TryFrom<&NuExpressionCustomValue> for NuExpression {
    type Error = ShellError;

    fn try_from(value: &NuExpressionCustomValue) -> Result<Self, Self::Error> {
        if let Some(expr) = &value.expr {
            Ok(expr.clone())
        } else {
            DataFrameCache::get_expr(&value.id)?.ok_or_else(|| ShellError::GenericError {
                error: format!("Expression {:?} not found in cache", value.id),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    }
}

impl From<NuExpression> for NuExpressionCustomValue {
    fn from(expr: NuExpression) -> Self {
        Self {
            id: expr.id,
            expr: Some(expr),
        }
    }
}

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuExpressionCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = self.clone();
        Value::custom_value(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        TYPE_NAME.into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let expr = NuExpression::try_from(self)?;
        expr.to_value(span)
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
        let expr = NuExpression::try_from(self)?;
        compute_with_value(&expr, lhs_span, operator, op, right)
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}

fn compute_with_value(
    left: &NuExpression,
    lhs_span: Span,
    operator: Operator,
    op: Span,
    right: &Value,
) -> Result<Value, ShellError> {
    let rhs_span = right.span();
    match right {
        Value::CustomValue { val: rhs, .. } => {
            let rhs = rhs.as_any().downcast_ref::<NuExpression>().ok_or_else(|| {
                ShellError::DowncastNotPossible {
                    msg: "Unable to create expression".into(),
                    span: rhs_span,
                }
            })?;

            match rhs.as_ref() {
                polars::prelude::Expr::Literal(..) => {
                    with_operator(operator, left, rhs, lhs_span, right.span(), op)
                }
                _ => Err(ShellError::TypeMismatch {
                    err_message: "Only literal expressions or number".into(),
                    span: right.span(),
                }),
            }
        }
        _ => {
            let rhs = NuExpression::try_from_value(right.clone())?;
            with_operator(operator, left, &rhs, lhs_span, right.span(), op)
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
            lhs_ty: Type::Custom(TYPE_NAME.into()).to_string(),
            lhs_span,
            rhs_ty: Type::Custom(TYPE_NAME.into()).to_string(),
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
