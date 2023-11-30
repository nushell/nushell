use crate::{
    ast::{
        eval_operator, Bits, Boolean, Call, Comparison, Expr, Expression, Math, Operator,
        RecordItem,
    },
    Record, ShellError, Span, Value, VarId,
};
use std::collections::HashMap;

pub trait Eval {
    type State<'a>: Copy;

    type MutState;

    fn eval(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        expr: &Expression,
    ) -> Result<Value, ShellError> {
        match &expr.expr {
            Expr::Bool(b) => Ok(Value::bool(*b, expr.span)),
            Expr::Int(i) => Ok(Value::int(*i, expr.span)),
            Expr::Float(f) => Ok(Value::float(*f, expr.span)),
            Expr::Binary(b) => Ok(Value::binary(b.clone(), expr.span)),
            Expr::Filepath(path) => Self::eval_filepath(state, mut_state, path.clone(), expr.span),
            Expr::Var(var_id) => Self::eval_var(state, mut_state, *var_id, expr.span),
            Expr::CellPath(cell_path) => Ok(Value::cell_path(cell_path.clone(), expr.span)),
            Expr::FullCellPath(_cell_path) => {
                // TODO: Both eval.rs and eval_const.rs seem to do the same thing, but
                // eval_const converts it to a generic error
                // (there's a todo saying to do better error conversion)
                // For now, perhaps they could both have the same implementation
                todo!()
            }
            Expr::DateTime(dt) => Ok(Value::date(*dt, expr.span)),
            Expr::List(x) => {
                let mut output = vec![];
                for expr in x {
                    match &expr.expr {
                        Expr::Spread(expr) => match Self::eval(state, mut_state, expr)? {
                            Value::List { mut vals, .. } => output.append(&mut vals),
                            _ => return Err(ShellError::CannotSpreadAsList { span: expr.span }),
                        },
                        _ => output.push(Self::eval(state, mut_state, expr)?),
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
                            let col_name = Self::value_as_string(
                                Self::eval(state, mut_state, col)?,
                                expr.span,
                            )?;
                            if let Some(orig_span) = col_names.get(&col_name) {
                                return Err(ShellError::ColumnDefinedTwice {
                                    col_name,
                                    second_use: col.span,
                                    first_use: *orig_span,
                                });
                            } else {
                                col_names.insert(col_name.clone(), col.span);
                                record.push(col_name, Self::eval(state, mut_state, val)?);
                            }
                        }
                        RecordItem::Spread(_, inner) => {
                            match Self::eval(state, mut_state, inner)? {
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
                                _ => {
                                    return Err(ShellError::CannotSpreadAsRecord {
                                        span: inner.span,
                                    })
                                }
                            }
                        }
                    }
                }

                Ok(Value::record(record, expr.span))
            }
            Expr::Table(headers, vals) => {
                let mut output_headers = vec![];
                for expr in headers {
                    let header =
                        Self::value_as_string(Self::eval(state, mut_state, expr)?, expr.span)?;
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
                        row.push(Self::eval(state, mut_state, expr)?);
                    }
                    // length equality already ensured in parser
                    output_rows.push(Value::record(
                        Record::from_raw_cols_vals(output_headers.clone(), row),
                        expr.span,
                    ));
                }
                Ok(Value::list(output_rows, expr.span))
            }
            Expr::Keyword(_, _, expr) => Self::eval(state, mut_state, expr),
            Expr::String(s) => Ok(Value::string(s.clone(), expr.span)),
            Expr::Nothing => Ok(Value::nothing(expr.span)),
            Expr::ValueWithUnit(_expr, _unit) => {
                // The two implementations seem to differ only in the error they give
                // when expr doesn't evaluate to an Int
                // eval.rs gives a CantConvert error, while eval_const says NotAConstant
                // CantConvert seems more appropriate for eval_const too, since the issue isn't that it's not
                // a constant, it's that it isn't an Int
                todo!()
            }
            Expr::Call(call) => Self::eval_call(state, mut_state, call, expr.span),
            Expr::ExternalCall(head, args, is_subexpression) => {
                Self::eval_external_call(state, mut_state, head, args, *is_subexpression)
            }
            Expr::Subexpression(_block_id) => todo!(),
            Expr::Range(from, next, to, operator) => {
                let from = if let Some(f) = from {
                    Self::eval(state, mut_state, f)?
                } else {
                    Value::Nothing {
                        internal_span: expr.span,
                    }
                };

                let next = if let Some(s) = next {
                    Self::eval(state, mut_state, s)?
                } else {
                    Value::Nothing {
                        internal_span: expr.span,
                    }
                };

                let to = if let Some(t) = to {
                    Self::eval(state, mut_state, t)?
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
                let lhs = Self::eval(state, mut_state, expr)?;
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
                let op = eval_operator(op)?;

                match op {
                    Operator::Boolean(boolean) => {
                        let lhs = Self::eval(state, mut_state, lhs)?;
                        match boolean {
                            Boolean::And => {
                                if lhs.is_false() {
                                    Ok(Value::bool(false, expr.span))
                                } else {
                                    let rhs = Self::eval(state, mut_state, rhs)?;
                                    lhs.and(op_span, &rhs, expr.span)
                                }
                            }
                            Boolean::Or => {
                                if lhs.is_true() {
                                    Ok(Value::bool(true, expr.span))
                                } else {
                                    let rhs = Self::eval(state, mut_state, rhs)?;
                                    lhs.or(op_span, &rhs, expr.span)
                                }
                            }
                            Boolean::Xor => {
                                let rhs = Self::eval(state, mut_state, rhs)?;
                                lhs.xor(op_span, &rhs, expr.span)
                            }
                        }
                    }
                    Operator::Math(math) => {
                        let lhs = Self::eval(state, mut_state, lhs)?;
                        let rhs = Self::eval(state, mut_state, rhs)?;

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
                    Operator::Comparison(comparison) => {
                        let lhs = Self::eval(state, mut_state, lhs)?;
                        let rhs = Self::eval(state, mut_state, rhs)?;
                        match comparison {
                            Comparison::LessThan => lhs.lt(op_span, &rhs, expr.span),
                            Comparison::LessThanOrEqual => lhs.lte(op_span, &rhs, expr.span),
                            Comparison::GreaterThan => lhs.gt(op_span, &rhs, expr.span),
                            Comparison::GreaterThanOrEqual => lhs.gte(op_span, &rhs, expr.span),
                            Comparison::Equal => lhs.eq(op_span, &rhs, expr.span),
                            Comparison::NotEqual => lhs.ne(op_span, &rhs, expr.span),
                            Comparison::In => lhs.r#in(op_span, &rhs, expr.span),
                            Comparison::NotIn => lhs.not_in(op_span, &rhs, expr.span),
                            Comparison::StartsWith => lhs.starts_with(op_span, &rhs, expr.span),
                            Comparison::EndsWith => lhs.ends_with(op_span, &rhs, expr.span),
                            Comparison::RegexMatch => todo!(),
                            Comparison::NotRegexMatch => todo!(),
                        }
                    }
                    Operator::Bits(bits) => {
                        let lhs = Self::eval(state, mut_state, lhs)?;
                        let rhs = Self::eval(state, mut_state, rhs)?;
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
            Expr::MatchPattern(_) => todo!(),
            Expr::MatchBlock(_) => todo!(),
            Expr::RowCondition(block_id) | Expr::Closure(block_id) => {
                Self::eval_row_condition_or_closure(state, mut_state, *block_id, expr.span)
            }
            Expr::StringInterpolation(_) => todo!(),
            Expr::Directory(_) => todo!(),
            Expr::GlobPattern(_) => todo!(),
            Expr::VarDecl(_) => todo!(),
            Expr::Signature(_) => todo!(),
            Expr::Spread(_) => todo!(),
            Expr::Operator(_) => todo!(),
            Expr::Garbage => todo!(),
        }
    }

    fn eval_filepath(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        path: String,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_var(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        var_id: VarId,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_call(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        call: &Call,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_external_call(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        head: &Expression,
        args: &[Expression],
        is_subexpression: bool,
    ) -> Result<Value, ShellError>;

    fn eval_row_condition_or_closure(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn value_as_string(value: Value, span: Span) -> Result<String, ShellError>;
}
