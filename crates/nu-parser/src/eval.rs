use nu_protocol::{
    ast::{Expr, Expression},
    engine::StateWorkingSet,
    ParseError, Span, Value,
};

/// Evaluate a constant value at parse time
///
/// Based off eval_expression() in the engine
pub fn eval_constant(
    working_set: &StateWorkingSet,
    expr: &Expression,
) -> Result<Value, ParseError> {
    match &expr.expr {
        Expr::Bool(b) => Ok(Value::bool(*b, expr.span)),
        Expr::Int(i) => Ok(Value::int(*i, expr.span)),
        Expr::Float(f) => Ok(Value::float(*f, expr.span)),
        Expr::Binary(b) => Ok(Value::Binary {
            val: b.clone(),
            span: expr.span,
        }),
        Expr::Filepath(path) => Ok(Value::String {
            val: path.clone(),
            span: expr.span,
        }),
        Expr::Var(var_id) => match working_set.find_constant(*var_id) {
            Some(val) => Ok(val.clone()),
            None => Err(ParseError::NotAConstant(expr.span)),
        },
        Expr::CellPath(cell_path) => Ok(Value::CellPath {
            val: cell_path.clone(),
            span: expr.span,
        }),
        Expr::FullCellPath(cell_path) => {
            let value = eval_constant(working_set, &cell_path.head)?;

            match value.follow_cell_path(&cell_path.tail, false) {
                Ok(val) => Ok(val),
                // TODO: Better error conversion
                Err(shell_error) => Err(ParseError::LabeledError(
                    "Error when following cell path".to_string(),
                    format!("{shell_error:?}"),
                    expr.span,
                )),
            }
        }
        Expr::DateTime(dt) => Ok(Value::Date {
            val: *dt,
            span: expr.span,
        }),
        Expr::List(x) => {
            let mut output = vec![];
            for expr in x {
                output.push(eval_constant(working_set, expr)?);
            }
            Ok(Value::List {
                vals: output,
                span: expr.span,
            })
        }
        Expr::Record(fields) => {
            let mut cols = vec![];
            let mut vals = vec![];
            for (col, val) in fields {
                // avoid duplicate cols.
                let col_name = value_as_string(eval_constant(working_set, col)?, expr.span)?;
                let pos = cols.iter().position(|c| c == &col_name);
                match pos {
                    Some(index) => {
                        vals[index] = eval_constant(working_set, val)?;
                    }
                    None => {
                        cols.push(col_name);
                        vals.push(eval_constant(working_set, val)?);
                    }
                }
            }

            Ok(Value::Record {
                cols,
                vals,
                span: expr.span,
            })
        }
        Expr::Table(headers, vals) => {
            let mut output_headers = vec![];
            for expr in headers {
                output_headers.push(value_as_string(
                    eval_constant(working_set, expr)?,
                    expr.span,
                )?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_constant(working_set, expr)?);
                }
                output_rows.push(Value::Record {
                    cols: output_headers.clone(),
                    vals: row,
                    span: expr.span,
                });
            }
            Ok(Value::List {
                vals: output_rows,
                span: expr.span,
            })
        }
        Expr::Keyword(_, _, expr) => eval_constant(working_set, expr),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Nothing => Ok(Value::Nothing { span: expr.span }),
        Expr::ValueWithUnit(expr, unit) => {
            if let Ok(Value::Int { val, .. }) = eval_constant(working_set, expr) {
                Ok(unit.item.to_value(val, unit.span))
            } else {
                Err(ParseError::NotAConstant(expr.span))
            }
        }
        _ => Err(ParseError::NotAConstant(expr.span)),
    }
}

/// Get the value as a string
pub fn value_as_string(value: Value, span: Span) -> Result<String, ParseError> {
    match value {
        Value::String { val, .. } => Ok(val),
        _ => Err(ParseError::NotAConstant(span)),
    }
}
