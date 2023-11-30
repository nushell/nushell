use crate::{
    ast::{
        eval_operator, Bits, Block, Boolean, Call, Comparison, Expr, Expression, Math, Operator,
        PipelineElement, RecordItem,
    },
    engine::{EngineState, StateWorkingSet},
    record, HistoryFileFormat, PipelineData, Range, Record, ShellError, Span, Value,
};
use nu_system::os_info::{get_kernel_version, get_os_arch, get_os_family, get_os_name};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub trait Eval {
    type State;

    fn eval(state: &Self::State, expr: &Expression) -> Result<Value, ShellError> {
        match &expr.expr {
            Expr::Bool(b) => Ok(Value::bool(*b, expr.span)),
            Expr::Int(i) => Ok(Value::int(*i, expr.span)),
            Expr::Float(f) => Ok(Value::float(*f, expr.span)),
            Expr::Binary(b) => Ok(Value::binary(b.clone(), expr.span)),
            Expr::Filepath(path) => todo!(),
            Expr::Var(var_id) => todo!(),
            Expr::CellPath(cell_path) => Ok(Value::cell_path(cell_path.clone(), expr.span)),
            Expr::FullCellPath(cell_path) => todo!(),
            Expr::DateTime(dt) => Ok(Value::date(*dt, expr.span)),
            Expr::List(x) => {
                let mut output = vec![];
                for expr in x {
                    match &expr.expr {
                        Expr::Spread(expr) => match Self::eval(state, expr)? {
                            Value::List { mut vals, .. } => output.append(&mut vals),
                            _ => return Err(ShellError::CannotSpreadAsList { span: expr.span }),
                        },
                        _ => output.push(Self::eval(state, expr)?),
                    }
                }
                Ok(Value::list(output, expr.span))
            }
            Expr::Record(items) => {
                let mut record = Record::new();
                let mut col_names = HashMap::new();
                for item in items {
                    match item {
                        RecordItem::Pair(col, val) => {
                            // avoid duplicate cols
                            let col_name =
                                Self::value_as_string(Self::eval(state, col)?, expr.span)?;
                            if let Some(orig_span) = col_names.get(&col_name) {
                                return Err(ShellError::ColumnDefinedTwice {
                                    col_name,
                                    second_use: col.span,
                                    first_use: *orig_span,
                                });
                            } else {
                                col_names.insert(col_name.clone(), col.span);
                                record.push(col_name, Self::eval(state, val)?);
                            }
                        }
                        RecordItem::Spread(_, inner) => match Self::eval(state, inner)? {
                            Value::Record { val: inner_val, .. } => {
                                for (col_name, val) in inner_val {
                                    if let Some(orig_span) = col_names.get(&col_name) {
                                        return Err(ShellError::ColumnDefinedTwice {
                                            col_name,
                                            second_use: inner.span,
                                            first_use: *orig_span,
                                        });
                                    } else {
                                        col_names.insert(col_name.clone(), inner.span);
                                        record.push(col_name, val);
                                    }
                                }
                            }
                            _ => return Err(ShellError::CannotSpreadAsRecord { span: inner.span }),
                        },
                    }
                }

                Ok(Value::record(record, expr.span))
            }
            Expr::Table(headers, vals) => {
                let mut output_headers = vec![];
                for expr in headers {
                    let header = Self::value_as_string(Self::eval(state, expr)?, expr.span)?;
                    if let Some(idx) = output_headers
                        .iter()
                        .position(|existing| existing == &header)
                    {
                        return Err(ShellError::ColumnDefinedTwice {
                            col_name: header,
                            second_use: expr.span,
                            first_use: headers[idx].span,
                        });
                    } else {
                        output_headers.push(header);
                    }
                }

                let mut output_rows = vec![];
                for val in vals {
                    let mut row = vec![];
                    for expr in val {
                        row.push(Self::eval(state, expr)?);
                    }
                    // length equality already ensured in parser
                    output_rows.push(Value::record(
                        Record::from_raw_cols_vals(output_headers.clone(), row),
                        expr.span,
                    ));
                }
                Ok(Value::list(output_rows, expr.span))
            }
            Expr::Keyword(_, _, expr) => Self::eval(state, expr),
            Expr::String(s) => Ok(Value::string(s.clone(), expr.span)),
            Expr::Nothing => Ok(Value::nothing(expr.span)),
            Expr::ValueWithUnit(expr, unit) => todo!(),
            Expr::Call(call) => todo!(),
            Expr::Subexpression(block_id) => todo!(),
            Expr::Range(from, next, to, operator) => {
                let from = if let Some(f) = from {
                    Self::eval(state, f)?
                } else {
                    Value::Nothing {
                        internal_span: expr.span,
                    }
                };

                let next = if let Some(s) = next {
                    Self::eval(state, s)?
                } else {
                    Value::Nothing {
                        internal_span: expr.span,
                    }
                };

                let to = if let Some(t) = to {
                    Self::eval(state, t)?
                } else {
                    Value::Nothing {
                        internal_span: expr.span,
                    }
                };
                Ok(Value::Range {
                    val: Box::new(crate::Range::new(expr.span, from, next, to, operator)?),
                    internal_span: expr.span,
                })
            }
            Expr::UnaryNot(expr) => {
                let lhs = Self::eval(state, expr)?;
                match lhs {
                    Value::Bool { val, .. } => Ok(Value::bool(!val, expr.span)),
                    other => Err(ShellError::TypeMismatch {
                        err_message: format!("expected bool, found {}", other.get_type()),
                        span: expr.span,
                    }),
                }
            }
            Expr::BinaryOp(lhs, op, rhs) => {
                let op_span = op.span;
                let op = Self::eval_operator(op)?;

                match op {
                    Operator::Boolean(boolean) => {
                        let lhs = Self::eval(state, lhs)?;
                        match boolean {
                            Boolean::And => {
                                if lhs.is_false() {
                                    Ok(Value::bool(false, expr.span))
                                } else {
                                    let rhs = Self::eval(state, rhs)?;
                                    lhs.and(op_span, &rhs, expr.span)
                                }
                            }
                            Boolean::Or => {
                                if lhs.is_true() {
                                    Ok(Value::bool(true, expr.span))
                                } else {
                                    let rhs = Self::eval(state, rhs)?;
                                    lhs.or(op_span, &rhs, expr.span)
                                }
                            }
                            Boolean::Xor => {
                                let rhs = Self::eval(state, rhs)?;
                                lhs.xor(op_span, &rhs, expr.span)
                            }
                        }
                    }
                    Operator::Math(math) => {
                        let lhs = Self::eval(state, lhs)?;
                        let rhs = Self::eval(state, rhs)?;

                        match math {
                            Math::Plus => lhs.add(op_span, &rhs, expr.span),
                            Math::Minus => lhs.sub(op_span, &rhs, expr.span),
                            Math::Multiply => lhs.mul(op_span, &rhs, expr.span),
                            Math::Divide => lhs.div(op_span, &rhs, expr.span),
                            Math::Append => lhs.append(op_span, &rhs, expr.span),
                            Math::Modulo => lhs.modulo(op_span, &rhs, expr.span),
                            Math::FloorDivision => lhs.floor_div(op_span, &rhs, expr.span),
                            Math::Pow => lhs.pow(op_span, &rhs, expr.span),
                        }
                    }
                    Operator::Comparison(comparison) => todo!(),
                    Operator::Bits(bits) => {
                        let lhs = Self::eval(state, lhs)?;
                        let rhs = Self::eval(state, rhs)?;
                        match bits {
                            Bits::BitAnd => lhs.bit_and(op_span, &rhs, expr.span),
                            Bits::BitOr => lhs.bit_or(op_span, &rhs, expr.span),
                            Bits::BitXor => lhs.bit_xor(op_span, &rhs, expr.span),
                            Bits::ShiftLeft => lhs.bit_shl(op_span, &rhs, expr.span),
                            Bits::ShiftRight => lhs.bit_shr(op_span, &rhs, expr.span),
                        }
                    }
                    Operator::Assignment(_) => todo!(),
                }
            }
            Expr::Block(block_id) => Ok(Value::block(*block_id, expr.span)),
            Expr::ImportPattern(_) => todo!(),
            Expr::Overlay(_) => todo!(),
            Expr::ExternalCall(_, _, _) => todo!(),
            Expr::MatchPattern(_) => todo!(),
            Expr::MatchBlock(_) => todo!(),
            Expr::RowCondition(_) => todo!(),
            Expr::StringInterpolation(_) => todo!(),
            Expr::Directory(_) => todo!(),
            Expr::GlobPattern(_) => todo!(),
            Expr::Signature(_) => todo!(),
            Expr::Spread(_) => todo!(),
            Expr::VarDecl(_) => todo!(),
            Expr::Operator(_) => todo!(),
            Expr::Closure(_) => todo!(),
            Expr::Garbage => todo!(),
            // _ => todo!(),
        }
    }

    fn value_as_string(value: Value, span: Span) -> Result<String, ShellError>;

    fn eval_operator(op: &Expression) -> Result<Operator, ShellError>;
}
