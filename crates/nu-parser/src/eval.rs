use crate::ParseError;
use nu_protocol::{
    ast::{Expr, Expression},
    engine::StateWorkingSet,
    Span, Value,
};

/// Determine the error to emit
#[derive(Copy, Clone)]
pub enum EvalMode<'a> {
    Assignment,         // ParseError::ValueNotAConstant
    Argument(&'a [u8]), // ParseError::ArgumentNotAConstant
}

/// Evaluate a constant value at parse time
///
/// Based off eval_expression() in the engine
pub fn eval_constant(
    working_set: &StateWorkingSet,
    expr: &Expression,
    mode: EvalMode<'_>,
) -> Result<Value, ParseError> {
    use EvalMode::*;
    let const_error = || match mode {
        Assignment => ParseError::ValueNotAConstant(expr.span),
        Argument(it) => {
            ParseError::ArgumentNotAConstant(String::from_utf8_lossy(it).into(), expr.span)
        }
    };

    match &expr.expr {
        Expr::Bool(b) => Ok(Value::boolean(*b, expr.span)),
        Expr::Int(i) => Ok(Value::int(*i, expr.span)),
        Expr::Float(f) => Ok(Value::float(*f, expr.span)),
        Expr::Binary(b) => Ok(Value::Binary {
            val: b.clone(),
            span: expr.span,
        }),
        Expr::Var(var_id) => match working_set.find_constant(*var_id) {
            Some(val) => Ok(val.clone()),
            None => Err(const_error()),
        },
        Expr::CellPath(cell_path) => Ok(Value::CellPath {
            val: cell_path.clone(),
            span: expr.span,
        }),
        Expr::FullCellPath(cell_path) => {
            let value = eval_constant(working_set, &cell_path.head, mode)?;

            match value.follow_cell_path(&cell_path.tail, false, false) {
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
                output.push(eval_constant(working_set, expr, mode)?);
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
                let col_name =
                    value_as_string(eval_constant(working_set, col, mode)?, expr.span, mode)?;
                let pos = cols.iter().position(|c| c == &col_name);
                match pos {
                    Some(index) => {
                        vals[index] = eval_constant(working_set, val, mode)?;
                    }
                    None => {
                        cols.push(col_name);
                        vals.push(eval_constant(working_set, val, mode)?);
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
                    eval_constant(working_set, expr, mode)?,
                    expr.span,
                    mode,
                )?);
            }

            let mut output_rows = vec![];
            for val in vals {
                let mut row = vec![];
                for expr in val {
                    row.push(eval_constant(working_set, expr, mode)?);
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
        Expr::Keyword(_, _, expr) => eval_constant(working_set, expr, mode),
        Expr::String(s) => Ok(Value::String {
            val: s.clone(),
            span: expr.span,
        }),
        Expr::Nothing => Ok(Value::Nothing { span: expr.span }),
        _ => Err(const_error()),
    }
}

/// Get the value as a string
pub fn value_as_string(value: Value, span: Span, mode: EvalMode) -> Result<String, ParseError> {
    use EvalMode::*;
    match value {
        Value::String { val, .. } => Ok(val),
        _ => Err(match mode {
            Assignment => ParseError::ValueNotAConstant(span),
            Argument(it) => {
                ParseError::ArgumentNotAConstant(String::from_utf8_lossy(it).into(), span)
            }
        }),
    }
}
