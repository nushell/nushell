use crate::{
    Cacheable, PolarsPlugin,
    values::{CustomValueSupport, PolarsPluginCustomValue, PolarsPluginType},
};
use std::ops::{Add, Div, Mul, Rem, Sub};

use super::NuExpression;
use nu_plugin::EngineInterface;
use nu_protocol::{
    CustomValue, ShellError, Span, Type, Value,
    ast::{Boolean, Comparison, Math, Operator},
};
use polars::prelude::Expr;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TYPE_NAME: &str = "polars_expression";

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
        PolarsPluginType::NuExpression.type_name().to_string()
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

fn with_operator(
    (plugin, engine): (&PolarsPlugin, &EngineInterface),
    operator: Operator,
    left: &NuExpression,
    right: &NuExpression,
    lhs_span: Span,
    _rhs_span: Span,
    op_span: Span,
) -> Result<Value, ShellError> {
    match operator {
        Operator::Math(Math::Add) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Add::add)
        }
        Operator::Math(Math::Subtract) => {
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
        Operator::Math(Math::FloorDivide) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Expr::floor_div)
        }
        Operator::Math(Math::Pow) => {
            apply_arithmetic(plugin, engine, left, right, lhs_span, Expr::pow)
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
        Operator::Boolean(Boolean::And) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::logical_and)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Boolean(Boolean::Or) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), Expr::logical_or)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        Operator::Boolean(Boolean::Xor) => Ok(left
            .clone()
            .apply_with_expr(right.clone(), logical_xor)
            .cache(plugin, engine, lhs_span)?
            .into_value(lhs_span)),
        op => Err(ShellError::OperatorUnsupportedType {
            op,
            unsupported: Type::Custom(TYPE_NAME.into()),
            op_span,
            unsupported_span: lhs_span,
            help: None,
        }),
    }
}

pub fn logical_xor(a: Expr, b: Expr) -> Expr {
    (a.clone().or(b.clone())) // A OR B
        .and((a.and(b)).not()) // AND with NOT (A AND B)
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
