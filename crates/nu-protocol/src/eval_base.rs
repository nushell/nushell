use crate::{
    ast::{
        eval_operator, Assignment, Bits, Boolean, Call, Comparison, Expr, Expression,
        ExternalArgument, Math, Operator, RecordItem,
    },
    debugger::DebugContext,
    Config, GetSpan, IntoInterruptiblePipelineData, Range, Record, ShellError, Span, Value, VarId,
};
use std::{borrow::Cow, collections::HashMap};

/// To share implementations for regular eval and const eval
pub trait Eval {
    /// State that doesn't need to be mutated.
    /// EngineState for regular eval and StateWorkingSet for const eval
    type State<'a>: Copy + GetSpan;

    /// State that needs to be mutated.
    /// This is the stack for regular eval, and unused by const eval
    type MutState;

    fn eval<D: DebugContext>(
        state: Self::State<'_>,
        mut_state: &mut Self::MutState,
        expr: &Expression,
    ) -> Result<Value, ShellError> {
        let expr_span = state.get_span(expr.span_id);

        match &expr.expr {
            Expr::Bool(b) => Ok(Value::bool(*b, expr_span)),
            Expr::Int(i) => Ok(Value::int(*i, expr_span)),
            Expr::Float(f) => Ok(Value::float(*f, expr_span)),
            Expr::Binary(b) => Ok(Value::binary(b.clone(), expr_span)),
            Expr::Filepath(path, quoted) => Self::eval_filepath(state, mut_state, path.clone(), *quoted, expr_span),
            Expr::Directory(path, quoted) => {
                Self::eval_directory(state, mut_state, path.clone(), *quoted, expr_span)
            }
            Expr::Var(var_id) => Self::eval_var(state, mut_state, *var_id, expr_span),
            Expr::CellPath(cell_path) => Ok(Value::cell_path(cell_path.clone(), expr_span)),
            Expr::FullCellPath(cell_path) => {
                let value = Self::eval::<D>(state, mut_state, &cell_path.head)?;

                value.follow_cell_path(&cell_path.tail, false)
            }
            Expr::DateTime(dt) => Ok(Value::date(*dt, expr_span)),
            Expr::List(x) => {
                let mut output = vec![];
                for expr in x {
                    match &expr.expr {
                        Expr::Spread(expr) => match Self::eval::<D>(state, mut_state, expr)? {
                            Value::List { mut vals, .. } => output.append(&mut vals),
                            _ => return Err(ShellError::CannotSpreadAsList { span: expr_span }),
                        },
                        _ => output.push(Self::eval::<D>(state, mut_state, expr)?),
                    }
                }
                Ok(Value::list(output, expr_span))
            }
            Expr::Record(items) => {
                let mut record = Record::new();
                let mut col_names = HashMap::new();
                for item in items {
                    match item {
                        RecordItem::Pair(col, val) => {
                            // avoid duplicate cols
                            let col_name = Self::eval::<D>(state, mut_state, col)?.coerce_into_string()?;
                            if let Some(orig_span) = col_names.get(&col_name) {
                                return Err(ShellError::ColumnDefinedTwice {
                                    col_name,
                                    second_use: state.get_span(col.span_id),
                                    first_use: *orig_span,
                                });
                            } else {
                                col_names.insert(col_name.clone(), state.get_span(col.span_id));
                                record.push(col_name, Self::eval::<D>(state, mut_state, val)?);
                            }
                        }
                        RecordItem::Spread(_, inner) => {
                            match Self::eval::<D>(state, mut_state, inner)? {
                                Value::Record { val: inner_val, .. } => {
                                    for (col_name, val) in *inner_val {
                                        if let Some(orig_span) = col_names.get(&col_name) {
                                            return Err(ShellError::ColumnDefinedTwice {
                                                col_name,
                                                second_use: state.get_span(inner.span_id),
                                                first_use: *orig_span,
                                            });
                                        } else {
                                            col_names.insert(col_name.clone(), state.get_span(inner.span_id));
                                            record.push(col_name, val);
                                        }
                                    }
                                }
                                _ => {
                                    return Err(ShellError::CannotSpreadAsRecord {
                                        span: state.get_span(inner.span_id),
                                    })
                                }
                            }
                        }
                    }
                }

                Ok(Value::record(record, expr_span))
            }
            Expr::Table(headers, vals) => {
                let mut output_headers = vec![];
                for expr in headers {
                    let header = Self::eval::<D>(state, mut_state, expr)?.coerce_into_string()?;
                    if let Some(idx) = output_headers
                        .iter()
                        .position(|existing| existing == &header)
                    {
                        return Err(ShellError::ColumnDefinedTwice {
                            col_name: header,
                            second_use: expr_span,
                            first_use: state.get_span(headers[idx].span_id),
                        });
                    } else {
                        output_headers.push(header);
                    }
                }

                let mut output_rows = vec![];
                for val in vals {
                    let record = output_headers.iter().zip(val).map(|(col, expr)| {
                        Self::eval::<D>(state, mut_state, expr).map(|val| (col.clone(), val))
                    }).collect::<Result<_,_>>()?;

                    output_rows.push(Value::record(
                        record,
                        expr_span,
                    ));
                }
                Ok(Value::list(output_rows, expr_span))
            }
            Expr::Keyword(_, _, expr) => Self::eval::<D>(state, mut_state, expr),
            Expr::String(s) => Ok(Value::string(s.clone(), expr_span)),
            Expr::Nothing => Ok(Value::nothing(expr_span)),
            Expr::ValueWithUnit(e, unit) => match Self::eval::<D>(state, mut_state, e)? {
                Value::Int { val, .. } => unit.item.build_value(val, unit.span),
                x => Err(ShellError::CantConvert {
                    to_type: "unit value".into(),
                    from_type: x.get_type().to_string(),
                    span: state.get_span(e.span_id),
                    help: None,
                }),
            },
            Expr::Call(call) => Self::eval_call::<D>(state, mut_state, call, expr_span),
            Expr::ExternalCall(head, args) => {
                Self::eval_external_call(state, mut_state, head, args, expr_span)
            }
            Expr::Subexpression(block_id) => {
                Self::eval_subexpression::<D>(state, mut_state, *block_id, expr_span)
            }
            Expr::Range(from, next, to, operator) => {
                let from = if let Some(f) = from {
                    Self::eval::<D>(state, mut_state, f)?
                } else {
                    Value::nothing(expr_span)
                };

                let next = if let Some(s) = next {
                    Self::eval::<D>(state, mut_state, s)?
                } else {
                    Value::nothing(expr_span)
                };

                let to = if let Some(t) = to {
                    Self::eval::<D>(state, mut_state, t)?
                } else {
                    Value::nothing(expr_span)
                };
                Ok(Value::range(
                    Range::new(expr_span, from, next, to, operator)?,
                    expr_span,
                ))
            }
            Expr::UnaryNot(expr) => {
                let lhs = Self::eval::<D>(state, mut_state, expr)?;
                match lhs {
                    Value::Bool { val, .. } => Ok(Value::bool(!val, expr_span)),
                    other => Err(ShellError::TypeMismatch {
                        err_message: format!("expected bool, found {}", other.get_type()),
                        span: expr_span,
                    }),
                }
            }
            Expr::BinaryOp(lhs, op, rhs) => {
                let op_span = state.get_span(op.span_id);
                let op = eval_operator(&state, op)?;

                match op {
                    Operator::Boolean(boolean) => {
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        match boolean {
                            Boolean::And => {
                                if lhs.is_false() {
                                    Ok(Value::bool(false, expr_span))
                                } else {
                                    let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                                    lhs.and(op_span, &rhs, expr_span)
                                }
                            }
                            Boolean::Or => {
                                if lhs.is_true() {
                                    Ok(Value::bool(true, expr_span))
                                } else {
                                    let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                                    lhs.or(op_span, &rhs, expr_span)
                                }
                            }
                            Boolean::Xor => {
                                let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                                lhs.xor(op_span, &rhs, expr_span)
                            }
                        }
                    }
                    Operator::Math(math) => {
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        let rhs = Self::eval::<D>(state, mut_state, rhs)?;

                        match math {
                            Math::Plus => lhs.add(op_span, &rhs, expr_span),
                            Math::Minus => lhs.sub(op_span, &rhs, expr_span),
                            Math::Multiply => lhs.mul(op_span, &rhs, expr_span),
                            Math::Divide => lhs.div(op_span, &rhs, expr_span),
                            Math::Append => lhs.append(op_span, &rhs, expr_span),
                            Math::Modulo => lhs.modulo(op_span, &rhs, expr_span),
                            Math::FloorDivision => lhs.floor_div(op_span, &rhs, expr_span),
                            Math::Pow => lhs.pow(op_span, &rhs, expr_span),
                        }
                    }
                    Operator::Comparison(comparison) => {
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                        match comparison {
                            Comparison::LessThan => lhs.lt(op_span, &rhs, expr_span),
                            Comparison::LessThanOrEqual => lhs.lte(op_span, &rhs, expr_span),
                            Comparison::GreaterThan => lhs.gt(op_span, &rhs, expr_span),
                            Comparison::GreaterThanOrEqual => lhs.gte(op_span, &rhs, expr_span),
                            Comparison::Equal => lhs.eq(op_span, &rhs, expr_span),
                            Comparison::NotEqual => lhs.ne(op_span, &rhs, expr_span),
                            Comparison::In => lhs.r#in(op_span, &rhs, expr_span),
                            Comparison::NotIn => lhs.not_in(op_span, &rhs, expr_span),
                            Comparison::StartsWith => lhs.starts_with(op_span, &rhs, expr_span),
                            Comparison::EndsWith => lhs.ends_with(op_span, &rhs, expr_span),
                            Comparison::RegexMatch => {
                                Self::regex_match(state, op_span, &lhs, &rhs, false, expr_span)
                            }
                            Comparison::NotRegexMatch => {
                                Self::regex_match(state, op_span, &lhs, &rhs, true, expr_span)
                            }
                        }
                    }
                    Operator::Bits(bits) => {
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                        match bits {
                            Bits::BitAnd => lhs.bit_and(op_span, &rhs, expr_span),
                            Bits::BitOr => lhs.bit_or(op_span, &rhs, expr_span),
                            Bits::BitXor => lhs.bit_xor(op_span, &rhs, expr_span),
                            Bits::ShiftLeft => lhs.bit_shl(op_span, &rhs, expr_span),
                            Bits::ShiftRight => lhs.bit_shr(op_span, &rhs, expr_span),
                        }
                    }
                    Operator::Assignment(assignment) => Self::eval_assignment::<D>(
                        state, mut_state, lhs, rhs, assignment, op_span, expr_span
                    ),
                }
            }
            Expr::Block(block_id) => Ok(Value::block(*block_id, expr_span)),
            Expr::RowCondition(block_id) | Expr::Closure(block_id) => {
                Self::eval_row_condition_or_closure(state, mut_state, *block_id, expr_span)
            }
            Expr::StringInterpolation(exprs) => {
                let mut parts = vec![];
                for expr in exprs {
                    parts.push(Self::eval::<D>(state, mut_state, expr)?);
                }

                let config = Self::get_config(state, mut_state);

                parts
                    .into_iter()
                    .into_pipeline_data(None)
                    .collect_string("", &config)
                    .map(|x| Value::string(x, expr_span))
            }
            Expr::Overlay(_) => Self::eval_overlay(state, expr_span),
            Expr::GlobPattern(pattern, quoted) => {
                // GlobPattern is similar to Filepath
                // But we don't want to expand path during eval time, it's required for `nu_engine::glob_from` to run correctly
                Ok(Value::glob(pattern, *quoted, expr_span))
            }
            Expr::MatchBlock(_) // match blocks are handled by `match`
            | Expr::VarDecl(_)
            | Expr::ImportPattern(_)
            | Expr::Signature(_)
            | Expr::Spread(_)
            | Expr::Operator(_)
            | Expr::Garbage => Self::unreachable(state, expr),
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

    fn eval_call<D: DebugContext>(
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
        span: Span,
    ) -> Result<Value, ShellError>;

    fn eval_subexpression<D: DebugContext>(
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

    #[allow(clippy::too_many_arguments)]
    fn eval_assignment<D: DebugContext>(
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
    fn unreachable(state: Self::State<'_>, expr: &Expression) -> Result<Value, ShellError>;
}
