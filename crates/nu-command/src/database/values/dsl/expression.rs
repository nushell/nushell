use nu_protocol::{
    ast::{Operator, PathMember},
    CustomValue, PipelineData, ShellError, Span, Type, Value,
};
use serde::{Deserialize, Serialize};
use sqlparser::ast::{BinaryOperator, Expr, Ident};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExprDb(Expr);

// Referenced access to the native expression
impl AsRef<Expr> for ExprDb {
    fn as_ref(&self) -> &Expr {
        &self.0
    }
}

impl AsMut<Expr> for ExprDb {
    fn as_mut(&mut self) -> &mut Expr {
        &mut self.0
    }
}

impl From<Expr> for ExprDb {
    fn from(expr: Expr) -> Self {
        Self(expr)
    }
}

impl CustomValue for ExprDb {
    fn clone_value(&self, span: Span) -> Value {
        let cloned = Self(self.0.clone());

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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn follow_path_int(&self, count: usize, span: Span) -> Result<Value, ShellError> {
        let path = PathMember::Int { val: count, span };

        ExprDb::expr_to_value(self.as_ref(), span).follow_cell_path(&[path], false)
    }

    fn follow_path_string(&self, column_name: String, span: Span) -> Result<Value, ShellError> {
        let path = PathMember::String {
            val: column_name,
            span,
        };

        ExprDb::expr_to_value(self.as_ref(), span).follow_cell_path(&[path], false)
    }

    fn typetag_name(&self) -> &'static str {
        "DB expresssion"
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }

    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        let right_expr = match right {
            Value::CustomValue { .. } => ExprDb::try_from_value(right).map(ExprDb::into_native),
            Value::String { val, .. } => Ok(Expr::Value(
                sqlparser::ast::Value::SingleQuotedString(val.clone()),
            )),
            Value::Int { val, .. } => Ok(Expr::Value(sqlparser::ast::Value::Number(
                format!("{}", val),
                false,
            ))),
            Value::Bool { val, .. } => Ok(Expr::Value(sqlparser::ast::Value::Boolean(*val))),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: Type::Custom(self.typetag_name().into()),
                lhs_span,
                rhs_ty: right.get_type(),
                rhs_span: right.span()?,
            }),
        }?;

        let sql_operator = match operator {
            Operator::Equal => Ok(BinaryOperator::Eq),
            Operator::NotEqual => Ok(BinaryOperator::NotEq),
            Operator::LessThan => Ok(BinaryOperator::Lt),
            Operator::GreaterThan => Ok(BinaryOperator::Gt),
            Operator::LessThanOrEqual => Ok(BinaryOperator::LtEq),
            Operator::GreaterThanOrEqual => Ok(BinaryOperator::GtEq),
            Operator::RegexMatch => Ok(BinaryOperator::PGRegexMatch),
            Operator::NotRegexMatch => Ok(BinaryOperator::PGRegexNotMatch),
            Operator::Plus => Ok(BinaryOperator::Plus),
            Operator::Minus => Ok(BinaryOperator::Minus),
            Operator::Multiply => Ok(BinaryOperator::Multiply),
            Operator::Divide => Ok(BinaryOperator::Divide),
            Operator::Modulo => Ok(BinaryOperator::Modulo),
            Operator::FloorDivision => Ok(BinaryOperator::Divide),
            Operator::And => Ok(BinaryOperator::And),
            Operator::Or => Ok(BinaryOperator::Or),
            Operator::In
            | Operator::NotIn
            | Operator::Pow
            | Operator::BitOr
            | Operator::BitXor
            | Operator::BitAnd
            | Operator::ShiftLeft
            | Operator::ShiftRight
            | Operator::StartsWith
            | Operator::EndsWith => Err(ShellError::UnsupportedOperator(operator, op)),
        }?;

        let expr = Expr::BinaryOp {
            left: Box::new(self.as_ref().clone()),
            op: sql_operator,
            right: Box::new(right_expr),
        };

        Ok(ExprDb(expr).into_value(lhs_span))
    }
}

impl ExprDb {
    pub fn try_from_value(value: &Value) -> Result<Self, ShellError> {
        match value {
            Value::CustomValue { val, span } => match val.as_any().downcast_ref::<Self>() {
                Some(expr) => Ok(Self(expr.0.clone())),
                None => Err(ShellError::CantConvert(
                    "db expression".into(),
                    "non-expression".into(),
                    *span,
                    None,
                )),
            },
            Value::String { val, .. } => Ok(Expr::Identifier(Ident {
                value: val.clone(),
                quote_style: None,
            })
            .into()),
            Value::Int { val, .. } => {
                Ok(Expr::Value(sqlparser::ast::Value::Number(format!("{}", val), false)).into())
            }
            x => Err(ShellError::CantConvert(
                "database".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, ShellError> {
        let value = input.into_value(span);
        Self::try_from_value(&value)
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::CustomValue {
            val: Box::new(self),
            span,
        }
    }

    pub fn into_native(self) -> Expr {
        self.0
    }

    pub fn to_value(&self, span: Span) -> Value {
        ExprDb::expr_to_value(self.as_ref(), span)
    }

    // Convenient function to extrac multiple Expr that could be inside a nushell Value
    pub fn extract_exprs(value: Value) -> Result<Vec<Expr>, ShellError> {
        ExtractedExpr::extract_exprs(value).map(ExtractedExpr::into_exprs)
    }
}

enum ExtractedExpr {
    Single(Expr),
    List(Vec<ExtractedExpr>),
}

impl ExtractedExpr {
    fn into_exprs(self) -> Vec<Expr> {
        match self {
            Self::Single(expr) => vec![expr],
            Self::List(exprs) => exprs
                .into_iter()
                .flat_map(ExtractedExpr::into_exprs)
                .collect(),
        }
    }

    fn extract_exprs(value: Value) -> Result<ExtractedExpr, ShellError> {
        match value {
            Value::String { val, .. } => {
                let expr = Expr::Identifier(Ident {
                    value: val,
                    quote_style: None,
                });

                Ok(ExtractedExpr::Single(expr))
            }
            Value::Int { val, .. } => {
                let expr = Expr::Value(sqlparser::ast::Value::Number(format!("{}", val), false));

                Ok(ExtractedExpr::Single(expr))
            }
            Value::Bool { val, .. } => {
                let expr = Expr::Value(sqlparser::ast::Value::Boolean(val));

                Ok(ExtractedExpr::Single(expr))
            }
            Value::CustomValue { .. } => {
                let expr = ExprDb::try_from_value(&value)?.into_native();
                Ok(ExtractedExpr::Single(expr))
            }
            Value::List { vals, .. } => vals
                .into_iter()
                .map(Self::extract_exprs)
                .collect::<Result<Vec<ExtractedExpr>, ShellError>>()
                .map(ExtractedExpr::List),
            x => Err(ShellError::CantConvert(
                "selection".into(),
                x.get_type().to_string(),
                x.span()?,
                None,
            )),
        }
    }
}

impl ExprDb {
    pub fn expr_to_value(expr: &Expr, span: Span) -> Value {
        match expr {
            Expr::Identifier(ident) => {
                let cols = vec!["value".into(), "quoted_style".into()];
                let val = Value::String {
                    val: ident.value.to_string(),
                    span,
                };
                let style = Value::String {
                    val: format!("{:?}", ident.quote_style),
                    span,
                };

                Value::Record {
                    cols,
                    vals: vec![val, style],
                    span,
                }
            }
            Expr::Value(value) => Value::String {
                val: format!("{}", value),
                span,
            },
            Expr::BinaryOp { left, op, right } => {
                let cols = vec!["left".into(), "op".into(), "right".into()];
                let left = ExprDb::expr_to_value(left.as_ref(), span);
                let right = ExprDb::expr_to_value(right.as_ref(), span);
                let op = Value::String {
                    val: format!("{}", op),
                    span,
                };

                let vals = vec![left, op, right];

                Value::Record { cols, vals, span }
            }
            Expr::Function(function) => {
                let cols = vec![
                    "name".into(),
                    "args".into(),
                    "over".into(),
                    "distinct".into(),
                ];
                let name = Value::String {
                    val: function.name.to_string(),
                    span,
                };

                let args: Vec<Value> = function
                    .args
                    .iter()
                    .map(|arg| Value::String {
                        val: arg.to_string(),
                        span,
                    })
                    .collect();
                let args = Value::List { vals: args, span };

                let over = Value::String {
                    val: format!("{:?}", function.over),
                    span,
                };

                let distinct = Value::Bool {
                    val: function.distinct,
                    span,
                };

                let vals = vec![name, args, over, distinct];
                Value::Record { cols, vals, span }
            }
            Expr::Nested(expr) => ExprDb::expr_to_value(expr, span),
            Expr::CompoundIdentifier(_) => todo!(),
            Expr::IsNull(_) => todo!(),
            Expr::IsNotNull(_) => todo!(),
            Expr::IsDistinctFrom(_, _) => todo!(),
            Expr::IsNotDistinctFrom(_, _) => todo!(),
            Expr::InList { .. } => todo!(),
            Expr::InSubquery { .. } => todo!(),
            Expr::InUnnest { .. } => todo!(),
            Expr::Between { .. } => todo!(),
            Expr::UnaryOp { .. } => todo!(),
            Expr::Cast { .. } => todo!(),
            Expr::TryCast { .. } => todo!(),
            Expr::Extract { .. } => todo!(),
            Expr::Substring { .. } => todo!(),
            Expr::Trim { .. } => todo!(),
            Expr::Collate { .. } => todo!(),
            Expr::TypedString { .. } => todo!(),
            Expr::MapAccess { .. } => todo!(),
            Expr::Case { .. } => todo!(),
            Expr::Exists(_) => todo!(),
            Expr::Subquery(_) => todo!(),
            Expr::ListAgg(_) => todo!(),
            Expr::GroupingSets(_) => todo!(),
            Expr::Cube(_) => todo!(),
            Expr::Rollup(_) => todo!(),
            Expr::Tuple(_) => todo!(),
            Expr::ArrayIndex { .. } => todo!(),
            Expr::Array(_) => todo!(),
        }
    }
}
