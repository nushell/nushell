use crate::{
    ast::{
        eval_operator, Assignment, Bits, Boolean, Call, Comparison, Expr, Expression,
        ExternalArgument, Math, Operator, RecordItem,
    },
    Config, IntoInterruptiblePipelineData, Range, Record, ShellError, Span, Value, VarId,
};
use std::{borrow::Cow, collections::HashMap};

/// To share implementations for regular eval and const eval
pub trait Eval {
    /// State that doesn't need to be mutated.
    /// EngineState for regular eval and StateWorkingSet for const eval
    type State<'a>: Copy;

    /// State that needs to be mutated.
    /// This is the stack for regular eval, and unused by const eval
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
            Expr::Filepath(path, quoted) => Self::eval_filepath(state, mut_state, path.clone(), *quoted, expr.span),
            Expr::Directory(path, quoted) => {
                Self::eval_directory(state, mut_state, path.clone(), *quoted, expr.span)
            }
            Expr::Var(var_id) => Self::eval_var(state, mut_state, *var_id, expr.span),
            Expr::CellPath(cell_path) => Ok(Value::cell_path(cell_path.clone(), expr.span)),
            Expr::FullCellPath(cell_path) => {
                let value = Self::eval(state, mut_state, &cell_path.head)?;

                value.follow_cell_path(&cell_path.tail, false)
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
                            let col_name = Self::eval(state, mut_state, col)?.as_string()?;
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
                    let header = Self::eval(state, mut_state, expr)?.as_string()?;
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
                        Record::from_raw_cols_vals_unchecked(output_headers.clone(), row),
                        expr.span,
                    ));
                }
                Ok(Value::list(output_rows, expr.span))
            }
            Expr::Keyword(_, _, expr) => Self::eval(state, mut_state, expr),
            Expr::String(s) => Ok(Value::string(s.clone(), expr.span)),
            Expr::Nothing => Ok(Value::nothing(expr.span)),
            Expr::ValueWithUnit(e, unit) => match Self::eval(state, mut_state, e)? {
                Value::Int { val, .. } => unit.item.to_value(val, unit.span),
                x => Err(ShellError::CantConvert {
                    to_type: "unit value".into(),
                    from_type: x.get_type().to_string(),
                    span: e.span,
                    help: None,
                }),
            },
            Expr::Call(call) => Self::eval_call(state, mut_state, call, expr.span),
            Expr::ExternalCall(head, args, is_subexpression) => {
                Self::eval_external_call(state, mut_state, head, args, *is_subexpression, expr.span)
            }
            Expr::Subexpression(block_id) => {
                Self::eval_subexpression(state, mut_state, *block_id, expr.span)
            }
            Expr::Range(from, next, to, operator) => {
                let from = if let Some(f) = from {
                    Self::eval(state, mut_state, f)?
                } else {
                    Value::nothing(expr.span)
                };

                let next = if let Some(s) = next {
                    Self::eval(state, mut_state, s)?
                } else {
                    Value::nothing(expr.span)
                };

                let to = if let Some(t) = to {
                    Self::eval(state, mut_state, t)?
                } else {
                    Value::nothing(expr.span)
                };
                Ok(Value::range(
                    Range::new(expr.span, from, next, to, operator)?,
                    expr.span,
                ))
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
                            Comparison::RegexMatch => {
                                Self::regex_match(state, op_span, &lhs, &rhs, false, expr.span)
                            }
                            Comparison::NotRegexMatch => {
                                Self::regex_match(state, op_span, &lhs, &rhs, true, expr.span)
                            }
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
                    Operator::Assignment(assignment) => Self::eval_assignment(
                        state, mut_state, lhs, rhs, assignment, op_span, expr.span,
                    ),
                }
            }
            Expr::Block(block_id) => Ok(Value::block(*block_id, expr.span)),
            Expr::RowCondition(block_id) | Expr::Closure(block_id) => {
                Self::eval_row_condition_or_closure(state, mut_state, *block_id, expr.span)
            }
            Expr::StringInterpolation(exprs) => {
                let mut parts = vec![];
                for expr in exprs {
                    parts.push(Self::eval(state, mut_state, expr)?);
                }

                let config = Self::get_config(state, mut_state);

                parts
                    .into_iter()
                    .into_pipeline_data(None)
                    .collect_string("", &config)
                    .map(|x| Value::string(x, expr.span))
            }
            Expr::Overlay(_) => Self::eval_overlay(state, expr.span),
            Expr::GlobPattern(pattern, quoted) => {
                // GlobPattern is similar to Filepath
                // But we don't want to expand path during eval time, it's required for `nu_engine::glob_from` to run correctly
                if *quoted {
                    Ok(Value::quoted_string(pattern, expr.span))
                } else {
                    Ok(Value::string(pattern, expr.span))
                }
            }
            Expr::MatchBlock(_) // match blocks are handled by `match`
            | Expr::VarDecl(_)
            | Expr::ImportPattern(_)
            | Expr::Signature(_)
            | Expr::Spread(_)
            | Expr::Operator(_)
            | Expr::Garbage => Self::unreachable(expr),
        }
    }

    fn get_config<'a>(state: Self::State<'a>, mut_state: &mut Self::MutState) -> Cow<'a, Config>;

    fn eval_filepath(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        path: String,
        quoted: bool,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_directory(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        path: String,
        quoted: bool,
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
        args: &[ExternalArgument],
        is_subexpression: bool,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_subexpression(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn regex_match(
        state: Self::State<'_>,
        op_span: Span,
        lhs: &Value,
        rhs: &Value,
        invert: bool,
        expr_span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_assignment(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        lhs: &Expression,
        rhs: &Expression,
        assignment: Assignment,
        op_span: Span,
        expr_span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_row_condition_or_closure(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        block_id: usize,
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_overlay(state: Self::State<'_>, span: Span) -> Result<Value, ShellError>;

    /// For expressions that should never actually be evaluated
    fn unreachable(expr: &Expression) -> Result<Value, ShellError>;
}
