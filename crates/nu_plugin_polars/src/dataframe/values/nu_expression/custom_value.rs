use crate::{
    values::{CustomValueSupport, PolarsPluginCustomValue},
    Cacheable, PolarsPlugin,
};
use std::ops::{Add, Div, Mul, Rem, Sub};

use super::NuExpression;
use nu_plugin::EngineInterface;
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

// CustomValue implementation for NuDataFrame
#[typetag::serde]
impl CustomValue for NuExpressionCustomValue {
    fn clone_value(&self, span: nu_protocol::Span) -> Value {
        let cloned = self.clone();
        Value::custom(Box::new(cloned), span)
    }

    fn type_name(&self) -> String {
        TYPE_NAME.into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            "NuExpressionCustomValue: custom_value_to_base_value should've been called",
            span,
        ))
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}

fn compute_with_value(
    (plugin, engine): (&PolarsPlugin, &EngineInterface),
    left: &NuExpression,
    lhs_span: Span,
    operator: Operator,
    op: Span,
    right: &Value,
) -> Result<Value, ShellError> {
    let rhs_span = right.span();
    match right {
        Value::Custom { val: rhs, .. } => {
            let rhs = rhs.as_any().downcast_ref::<NuExpression>().ok_or_else(|| {
                ShellError::TypeMismatch {
                    err_message: "Right hand side not a dataframe expression".into(),
                    span: rhs_span,
                }
            })?;

            match rhs.as_ref() {
                polars::prelude::Expr::Literal(..) => with_operator(
                    (plugin, engine),
                    operator,
                    left,
                    rhs,
                    lhs_span,
                    right.span(),
                    op,
                ),
                _ => Err(ShellError::TypeMismatch {
                    err_message: "Only literal expressions or number".into(),
                    span: right.span(),
                }),
            }
        }
        _ => {
            let rhs = NuExpression::try_from_value(plugin, right)?;
            with_operator(
                (plugin, engine),
                operator,
                left,
                &rhs,
                lhs_span,
                right.span(),
                op,
            )
        }
    }
}

fn with_operator(
    (plugin, engine): (&PolarsPlugin, &EngineInterface),
    operator: Operator,
    left: &NuExpression,
    right: &NuExpression,
    lhs_span: Span,
    rhs_span: Span,
    op_span: Span,
) -> Result<Value, ShellError> {
    match operator {
        Operator::Math(Math::Plus) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Add::add)
        }
        Operator::Math(Math::Minus) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Sub::sub)
        }
        Operator::Math(Math::Multiply) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Mul::mul)
        }
        Operator::Math(Math::Divide) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Div::div)
        }
        Operator::Math(Math::Modulo) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Rem::rem)
        }
        Operator::Math(Math::FloorDivision) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Div::div)
        }
        Operator::Comparison(Comparison::Equal) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::eq)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::NotEqual) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::neq)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::GreaterThan) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::gt)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::GreaterThanOrEqual) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::gt_eq)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::LessThan) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::lt)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Comparison(Comparison::LessThanOrEqual) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::lt_eq)
            .cache(plugin, engine, lhs_span)?
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
    plugin: &PolarsPlugin,
    engine: &EngineInterface,
    left: &NuExpression,
    right: &NuExpression,
    span: Span,
    f: F,
) -> Result<Value, ShellError>
where
    F: Fn(Expr, Expr) -> Expr,
{
    let expr: NuExpression = f(left.as_ref().clone(), right.as_ref().clone()).into();

    Ok(expr.cache(plugin, engine, span)?.into_value(span))
}

impl PolarsPluginCustomValue for NuExpressionCustomValue {
    type PolarsPluginObjectType = NuExpression;

    fn custom_value_operation(
        &self,
        plugin: &crate::PolarsPlugin,
        engine: &nu_plugin::EngineInterface,
        lhs_span: Span,
        operator: nu_protocol::Spanned<nu_protocol::ast::Operator>,
        right: Value,
    ) -> Result<Value, ShellError> {
        let expr = NuExpression::try_from_custom_value(plugin, self)?;
        compute_with_value(
            (plugin, engine),
            &expr,
            lhs_span,
            operator.item,
            operator.span,
            &right,
        )
    }

    fn custom_value_to_base_value(
        &self,
        plugin: &crate::PolarsPlugin,
        _engine: &nu_plugin::EngineInterface,
    ) -> Result<Value, ShellError> {
        let expr = NuExpression::try_from_custom_value(plugin, self)?;
        expr.base_value(Span::unknown())
    }

    fn id(&self) -> &Uuid {
        &self.id
    }

    fn internal(&self) -> &Option<Self::PolarsPluginObjectType> {
        &self.expr
    }
}
