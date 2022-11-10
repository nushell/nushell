use polars::error::PolarsError;
use polars::prelude::{col, lit, DataType, Expr, LiteralValue, PolarsResult as Result, TimeUnit};

use sqlparser::ast::{
    BinaryOperator as SQLBinaryOperator, DataType as SQLDataType, Expr as SqlExpr,
    Function as SQLFunction, Value as SqlValue, WindowSpec,
};

fn map_sql_polars_datatype(data_type: &SQLDataType) -> Result<DataType> {
    Ok(match data_type {
        SQLDataType::Char(_)
        | SQLDataType::Varchar(_)
        | SQLDataType::Uuid
        | SQLDataType::Clob(_)
        | SQLDataType::Text
        | SQLDataType::String => DataType::Utf8,
        SQLDataType::Float(_) => DataType::Float32,
        SQLDataType::Real => DataType::Float32,
        SQLDataType::Double => DataType::Float64,
        SQLDataType::TinyInt(_) => DataType::Int8,
        SQLDataType::UnsignedTinyInt(_) => DataType::UInt8,
        SQLDataType::SmallInt(_) => DataType::Int16,
        SQLDataType::UnsignedSmallInt(_) => DataType::UInt16,
        SQLDataType::Int(_) => DataType::Int32,
        SQLDataType::UnsignedInt(_) => DataType::UInt32,
        SQLDataType::BigInt(_) => DataType::Int64,
        SQLDataType::UnsignedBigInt(_) => DataType::UInt64,

        SQLDataType::Boolean => DataType::Boolean,
        SQLDataType::Date => DataType::Date,
        SQLDataType::Time => DataType::Time,
        SQLDataType::Timestamp => DataType::Datetime(TimeUnit::Milliseconds, None),
        SQLDataType::Interval => DataType::Duration(TimeUnit::Milliseconds),
        SQLDataType::Array(inner_type) => {
            DataType::List(Box::new(map_sql_polars_datatype(inner_type)?))
        }
        _ => {
            return Err(PolarsError::ComputeError(
                format!(
                    "SQL Datatype {:?} was not supported in polars-sql yet!",
                    data_type
                )
                .into(),
            ))
        }
    })
}

fn cast_(expr: Expr, data_type: &SQLDataType) -> Result<Expr> {
    let polars_type = map_sql_polars_datatype(data_type)?;
    Ok(expr.cast(polars_type))
}

fn binary_op_(left: Expr, right: Expr, op: &SQLBinaryOperator) -> Result<Expr> {
    Ok(match op {
        SQLBinaryOperator::Plus => left + right,
        SQLBinaryOperator::Minus => left - right,
        SQLBinaryOperator::Multiply => left * right,
        SQLBinaryOperator::Divide => left / right,
        SQLBinaryOperator::Modulo => left % right,
        SQLBinaryOperator::StringConcat => left.cast(DataType::Utf8) + right.cast(DataType::Utf8),
        SQLBinaryOperator::Gt => left.gt(right),
        SQLBinaryOperator::Lt => left.lt(right),
        SQLBinaryOperator::GtEq => left.gt_eq(right),
        SQLBinaryOperator::LtEq => left.lt_eq(right),
        SQLBinaryOperator::Eq => left.eq(right),
        SQLBinaryOperator::NotEq => left.eq(right).not(),
        SQLBinaryOperator::And => left.and(right),
        SQLBinaryOperator::Or => left.or(right),
        SQLBinaryOperator::Xor => left.xor(right),
        _ => {
            return Err(PolarsError::ComputeError(
                format!("SQL Operator {:?} was not supported in polars-sql yet!", op).into(),
            ))
        }
    })
}

fn literal_expr(value: &SqlValue) -> Result<Expr> {
    Ok(match value {
        SqlValue::Number(s, _) => {
            // Check for existence of decimal separator dot
            if s.contains('.') {
                s.parse::<f64>().map(lit).map_err(|_| {
                    PolarsError::ComputeError(format!("Can't parse literal {:?}", s).into())
                })
            } else {
                s.parse::<i64>().map(lit).map_err(|_| {
                    PolarsError::ComputeError(format!("Can't parse literal {:?}", s).into())
                })
            }?
        }
        SqlValue::SingleQuotedString(s) => lit(s.clone()),
        SqlValue::NationalStringLiteral(s) => lit(s.clone()),
        SqlValue::HexStringLiteral(s) => lit(s.clone()),
        SqlValue::DoubleQuotedString(s) => lit(s.clone()),
        SqlValue::Boolean(b) => lit(*b),
        SqlValue::Null => Expr::Literal(LiteralValue::Null),
        _ => {
            return Err(PolarsError::ComputeError(
                format!(
                    "Parsing SQL Value {:?} was not supported in polars-sql yet!",
                    value
                )
                .into(),
            ))
        }
    })
}

pub fn parse_sql_expr(expr: &SqlExpr) -> Result<Expr> {
    Ok(match expr {
        SqlExpr::Identifier(e) => col(&e.value),
        SqlExpr::BinaryOp { left, op, right } => {
            let left = parse_sql_expr(left)?;
            let right = parse_sql_expr(right)?;
            binary_op_(left, right, op)?
        }
        SqlExpr::Function(sql_function) => parse_sql_function(sql_function)?,
        SqlExpr::Cast { expr, data_type } => cast_(parse_sql_expr(expr)?, data_type)?,
        SqlExpr::Nested(expr) => parse_sql_expr(expr)?,
        SqlExpr::Value(value) => literal_expr(value)?,
        _ => {
            return Err(PolarsError::ComputeError(
                format!(
                    "Expression: {:?} was not supported in polars-sql yet!",
                    expr
                )
                .into(),
            ))
        }
    })
}

fn apply_window_spec(expr: Expr, window_spec: &Option<WindowSpec>) -> Result<Expr> {
    Ok(match &window_spec {
        Some(window_spec) => {
            // Process for simple window specification, partition by first
            let partition_by = window_spec
                .partition_by
                .iter()
                .map(parse_sql_expr)
                .collect::<Result<Vec<_>>>()?;
            expr.over(partition_by)
            // Order by and Row range may not be supported at the moment
        }
        None => expr,
    })
}

fn parse_sql_function(sql_function: &SQLFunction) -> Result<Expr> {
    use sqlparser::ast::{FunctionArg, FunctionArgExpr};
    // Function name mostly do not have name space, so it mostly take the first args
    let function_name = sql_function.name.0[0].value.to_lowercase();
    let args = sql_function
        .args
        .iter()
        .map(|arg| match arg {
            FunctionArg::Named { arg, .. } => arg,
            FunctionArg::Unnamed(arg) => arg,
        })
        .collect::<Vec<_>>();
    Ok(
        match (
            function_name.as_str(),
            args.as_slice(),
            sql_function.distinct,
        ) {
            ("sum", [FunctionArgExpr::Expr(expr)], false) => {
                apply_window_spec(parse_sql_expr(expr)?, &sql_function.over)?.sum()
            }
            ("count", [FunctionArgExpr::Expr(expr)], false) => {
                apply_window_spec(parse_sql_expr(expr)?, &sql_function.over)?.count()
            }
            ("count", [FunctionArgExpr::Expr(expr)], true) => {
                apply_window_spec(parse_sql_expr(expr)?, &sql_function.over)?.n_unique()
            }
            // Special case for wildcard args to count function.
            ("count", [FunctionArgExpr::Wildcard], false) => lit(1i32).count(),
            _ => {
                return Err(PolarsError::ComputeError(
                    format!(
                        "Function {:?} with args {:?} was not supported in polars-sql yet!",
                        function_name, args
                    )
                    .into(),
                ))
            }
        },
    )
}
