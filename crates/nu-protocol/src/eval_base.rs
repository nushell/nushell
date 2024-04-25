use crate::{
    ast::{
        eval_operator, Assignment, Bits, Boolean, Call, Comparison, Expr, Expression,
        ExternalArgument, ListItem, Math, Operator, RecordItem,
    },
    debugger::DebugContext,
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

    fn eval<D: DebugContext>(
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
                let value = Self::eval::<D>(state, mut_state, &cell_path.head)?;

                value.follow_cell_path(&cell_path.tail, false)
            }
            Expr::DateTime(dt) => Ok(Value::date(*dt, expr.span)),
            Expr::List(list) => {
                let mut output = vec![];
                for item in list {
                    match item {
                        ListItem::Item(expr) => output.push(Self::eval::<D>(state, mut_state, expr)?),
                        ListItem::Spread(_, expr) => match Self::eval::<D>(state, mut_state, expr)? {
                            Value::List { vals, .. } => output.extend(vals),
                            _ => return Err(ShellError::CannotSpreadAsList { span: expr.span }),
                        },
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
                            let col_name = Self::eval::<D>(state, mut_state, col)?.coerce_into_string()?;
                            if let Some(orig_span) = col_names.get(&col_name) {
                                return Err(ShellError::ColumnDefinedTwice {
                                    col_name,
                                    second_use: col.span,
                                    first_use: *orig_span,
                                });
                            } else {
                                col_names.insert(col_name.clone(), col.span);
                                record.push(col_name, Self::eval::<D>(state, mut_state, val)?);
                            }
                        }
                        RecordItem::Spread(_, inner) => {
                            match Self::eval::<D>(state, mut_state, inner)? {
                                Value::Record { val: inner_val, .. } => {
                                    for (col_name, val) in inner_val.into_owned() {
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
            Expr::Table(table) => {
                let mut output_headers = vec![];
                for expr in table.columns.as_ref() {
                    let header = Self::eval::<D>(state, mut_state, expr)?.coerce_into_string()?;
                    if let Some(idx) = output_headers
                        .iter()
                        .position(|existing| existing == &header)
                    {
                        return Err(ShellError::ColumnDefinedTwice {
                            col_name: header,
                            second_use: expr.span,
                            first_use: table.columns[idx].span,
                        });
                    } else {
                        output_headers.push(header);
                    }
                }

                let mut output_rows = vec![];
                for val in table.rows.as_ref() {
                    let record = output_headers.iter().zip(val.as_ref()).map(|(col, expr)| {
                        Self::eval::<D>(state, mut_state, expr).map(|val| (col.clone(), val))
                    }).collect::<Result<_,_>>()?;

                    output_rows.push(Value::record(
                        record,
                        expr.span,
                    ));
                }
                Ok(Value::list(output_rows, expr.span))
            }
            Expr::Keyword(kw) => Self::eval::<D>(state, mut_state, &kw.expr),
            Expr::String(s) => Ok(Value::string(s.clone(), expr.span)),
            Expr::Nothing => Ok(Value::nothing(expr.span)),
            Expr::ValueWithUnit(value) => match Self::eval::<D>(state, mut_state, &value.expr)? {
                Value::Int { val, .. } => value.unit.item.build_value(val, value.unit.span),
                x => Err(ShellError::CantConvert {
                    to_type: "unit value".into(),
                    from_type: x.get_type().to_string(),
                    span: value.expr.span,
                    help: None,
                }),
            },
            Expr::Call(call) => Self::eval_call::<D>(state, mut_state, call, expr.span),
            Expr::ExternalCall(head, args) => {
                Self::eval_external_call(state, mut_state, head, args, expr.span)
            }
            Expr::Subexpression(block_id) => {
                Self::eval_subexpression::<D>(state, mut_state, *block_id, expr.span)
            }
            Expr::Range(range) => {
                let from = if let Some(f) = &range.from {
                    Self::eval::<D>(state, mut_state, f)?
                } else {
                    Value::nothing(expr.span)
                };

                let next = if let Some(s) = &range.next {
                    Self::eval::<D>(state, mut_state, s)?
                } else {
                    Value::nothing(expr.span)
                };

                let to = if let Some(t) = &range.to {
                    Self::eval::<D>(state, mut_state, t)?
                } else {
                    Value::nothing(expr.span)
                };

                Ok(Value::range(
                    Range::new(from, next, to, range.operator.inclusion, expr.span)?,
                    expr.span,
                ))
            }
            Expr::UnaryNot(expr) => {
                let lhs = Self::eval::<D>(state, mut_state, expr)?;
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
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        match boolean {
                            Boolean::And => {
                                if lhs.is_false() {
                                    Ok(Value::bool(false, expr.span))
                                } else {
                                    let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                                    lhs.and(op_span, &rhs, expr.span)
                                }
                            }
                            Boolean::Or => {
                                if lhs.is_true() {
                                    Ok(Value::bool(true, expr.span))
                                } else {
                                    let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                                    lhs.or(op_span, &rhs, expr.span)
                                }
                            }
                            Boolean::Xor => {
                                let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                                lhs.xor(op_span, &rhs, expr.span)
                            }
                        }
                    }
                    Operator::Math(math) => {
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        let rhs = Self::eval::<D>(state, mut_state, rhs)?;

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
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        let rhs = Self::eval::<D>(state, mut_state, rhs)?;
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
                        let lhs = Self::eval::<D>(state, mut_state, lhs)?;
                        let rhs = Self::eval::<D>(state, mut_state, rhs)?;
                        match bits {
                            Bits::BitAnd => lhs.bit_and(op_span, &rhs, expr.span),
                            Bits::BitOr => lhs.bit_or(op_span, &rhs, expr.span),
                            Bits::BitXor => lhs.bit_xor(op_span, &rhs, expr.span),
                            Bits::ShiftLeft => lhs.bit_shl(op_span, &rhs, expr.span),
                            Bits::ShiftRight => lhs.bit_shr(op_span, &rhs, expr.span),
                        }
                    }
                    Operator::Assignment(assignment) => Self::eval_assignment::<D>(
                        state, mut_state, lhs, rhs, assignment, op_span, expr.span
                    ),
                }
            }
            Expr::RowCondition(block_id) | Expr::Closure(block_id) => {
                Self::eval_row_condition_or_closure(state, mut_state, *block_id, expr.span)
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
                    .map(|x| Value::string(x, expr.span))
            }
            Expr::Overlay(_) => Self::eval_overlay(state, expr.span),
            Expr::GlobPattern(pattern, quoted) => {
                // GlobPattern is similar to Filepath
                // But we don't want to expand path during eval time, it's required for `nu_engine::glob_from` to run correctly
                Ok(Value::glob(pattern, *quoted, expr.span))
            }
            Expr::MatchBlock(_) // match blocks are handled by `match`
            | Expr::Block(_) // blocks are handled directly by core commands
            | Expr::VarDecl(_)
            | Expr::ImportPattern(_)
            | Expr::Signature(_)
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
    fn unreachable(expr: &Expression) -> Result<Value, ShellError>;
}
