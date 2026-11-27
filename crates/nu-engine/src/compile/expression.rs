use super::{
    BlockBuilder, CompileError, RedirectModes, compile_binary_op, compile_block, compile_call,
    compile_external_call, compile_load_env,
};

use nu_protocol::{
    ENV_VARIABLE_ID, IN_VARIABLE_ID, IntoSpanned, RegId, Value, VarId,
    ast::{
        CellPath, Expr, Expression, ListItem, Math, Operator, RecordItem, Traverse, ValueWithUnit,
    },
    engine::StateWorkingSet,
    ir::{DataSlice, Instruction, Literal},
};
use std::{cell::Cell, ops::ControlFlow};

pub(crate) fn compile_expression(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    expr: &Expression,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    let drop_input = |builder: &mut BlockBuilder| {
        if let Some(in_reg) = in_reg
            && in_reg != out_reg
        {
            builder.drop_reg(in_reg)?;
        }
        Ok(())
    };

    let lit = |builder: &mut BlockBuilder, literal: Literal| {
        drop_input(builder)?;

        builder
            .push(
                Instruction::LoadLiteral {
                    dst: out_reg,
                    lit: literal,
                }
                .into_spanned(expr.span),
            )
            .map(|_| ())
    };

    let ignore = |builder: &mut BlockBuilder| {
        drop_input(builder)?;
        builder.load_empty(out_reg)
    };

    let unexpected = |expr_name: &str| CompileError::UnexpectedExpression {
        expr_name: expr_name.into(),
        span: expr.span,
    };

    let move_in_reg_to_out_reg = |builder: &mut BlockBuilder| {
        // Ensure that out_reg contains the input value, because a call only uses one register
        if let Some(in_reg) = in_reg {
            if in_reg != out_reg {
                // Have to move in_reg to out_reg so it can be used
                builder.push(
                    Instruction::Move {
                        dst: out_reg,
                        src: in_reg,
                    }
                    .into_spanned(expr.span),
                )?;
            }
        } else {
            // Will have to initialize out_reg with Empty first
            builder.load_empty(out_reg)?;
        }
        Ok(())
    };

    match &expr.expr {
        Expr::AttributeBlock(ab) => compile_expression(
            working_set,
            builder,
            &ab.item,
            redirect_modes,
            in_reg,
            out_reg,
        ),
        Expr::Bool(b) => lit(builder, Literal::Bool(*b)),
        Expr::Int(i) => lit(builder, Literal::Int(*i)),
        Expr::Float(f) => lit(builder, Literal::Float(*f)),
        Expr::Binary(bin) => {
            let data_slice = builder.data(bin)?;
            lit(builder, Literal::Binary(data_slice))
        }
        Expr::Range(range) => {
            // Compile the subexpressions of the range
            let compile_part = |builder: &mut BlockBuilder,
                                part_expr: Option<&Expression>|
             -> Result<RegId, CompileError> {
                let reg = builder.next_register()?;
                if let Some(part_expr) = part_expr {
                    compile_expression(
                        working_set,
                        builder,
                        part_expr,
                        RedirectModes::value(part_expr.span),
                        None,
                        reg,
                    )?;
                } else {
                    builder.load_literal(reg, Literal::Nothing.into_spanned(expr.span))?;
                }
                Ok(reg)
            };

            drop_input(builder)?;

            let start = compile_part(builder, range.from.as_ref())?;
            let step = compile_part(builder, range.next.as_ref())?;
            let end = compile_part(builder, range.to.as_ref())?;

            // Assemble the range
            builder.load_literal(
                out_reg,
                Literal::Range {
                    start,
                    step,
                    end,
                    inclusion: range.operator.inclusion,
                }
                .into_spanned(expr.span),
            )
        }
        Expr::Var(var_id) if builder.stream_input_vars.contains(var_id) => {
            // The one read of a `$in` that wasn't collected: the input goes on as it is. It takes
            // the span of `$in`, as loading the variable would, so errors still point there. An
            // external's output collected later loses its last newline, as collecting `$in` did.
            move_in_reg_to_out_reg(builder)?;
            builder.push(Instruction::Span { src_dst: out_reg }.into_spanned(expr.span))?;
            builder.push(Instruction::TrimEndNewline { src_dst: out_reg }.into_spanned(expr.span))
        }
        Expr::Var(var_id) => {
            // Special case: $in should load from input register if available.
            // When the input register is present, we store it as $in for user access.
            if let Some(in_reg) = in_reg
                && *var_id == IN_VARIABLE_ID
            {
                // Clone input to a temporary register so it can be stored and loaded
                let cloned_reg = builder.clone_reg(in_reg, expr.span)?;
                builder.push(
                    Instruction::StoreVariable {
                        var_id: IN_VARIABLE_ID,
                        src: cloned_reg,
                    }
                    .into_spanned(expr.span),
                )?;
                // Now load it into output register
                builder.push(
                    Instruction::LoadVariable {
                        dst: out_reg,
                        var_id: IN_VARIABLE_ID,
                        preserve_origin: false,
                    }
                    .into_spanned(expr.span),
                )?;
            } else {
                drop_input(builder)?;
                builder.push(
                    Instruction::LoadVariable {
                        dst: out_reg,
                        var_id: *var_id,
                        preserve_origin: false,
                    }
                    .into_spanned(expr.span),
                )?;
            }
            Ok(())
        }
        Expr::VarDecl(_) => Err(unexpected("VarDecl")),
        Expr::Pattern(_) => Err(unexpected("Pattern")), // only for `let`, `mut` and `for`
        Expr::AssignmentPattern(_) => Err(unexpected("AssignmentPattern")), // only for `=`
        Expr::Call(call) => {
            move_in_reg_to_out_reg(builder)?;

            compile_call(working_set, builder, call, redirect_modes, in_reg, out_reg)
        }
        Expr::ExternalCall(head, args) => {
            move_in_reg_to_out_reg(builder)?;

            compile_external_call(
                working_set,
                builder,
                head,
                args,
                redirect_modes,
                in_reg,
                out_reg,
            )
        }
        Expr::Operator(_) => Err(unexpected("Operator")),
        Expr::RowCondition(block_id) => lit(builder, Literal::RowCondition(*block_id)),
        Expr::UnaryNot(subexpr) => {
            drop_input(builder)?;
            compile_expression(
                working_set,
                builder,
                subexpr,
                RedirectModes::value(subexpr.span),
                None,
                out_reg,
            )?;
            builder.push(Instruction::Not { src_dst: out_reg }.into_spanned(expr.span))?;
            Ok(())
        }
        Expr::UnaryMinus(operand) => {
            drop_input(builder)?;
            compile_expression(
                working_set,
                builder,
                operand,
                RedirectModes::value(operand.span),
                None,
                out_reg,
            )?;
            // Negate by multiplying by -1, which works for every type that can be negated
            // (ints, floats, durations and filesizes).
            let minus_one = builder.literal(Literal::Int(-1).into_spanned(expr.span))?;
            builder.push(
                Instruction::BinaryOp {
                    lhs_dst: out_reg,
                    op: Operator::Math(Math::Multiply),
                    rhs: minus_one,
                }
                .into_spanned(expr.span),
            )?;
            Ok(())
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            if let Expr::Operator(operator) = op.expr {
                // Don't drop input if the expression uses $in
                if !expr.has_in_variable(working_set) {
                    drop_input(builder)?;
                }
                compile_binary_op(
                    working_set,
                    builder,
                    lhs,
                    operator.into_spanned(op.span),
                    rhs,
                    expr.span,
                    in_reg,
                    out_reg,
                )
            } else {
                Err(CompileError::UnsupportedOperatorExpression { span: op.span })
            }
        }
        Expr::Collect(var_id, expr)
            if in_reg.is_some() && reads_input_once_at_head(working_set, *var_id, expr) =>
        {
            // Nothing else reads this `$in`, so there's no value to share: pass the input
            // through, stream and all, as if the block had used its input directly.
            builder.stream_input_vars.push(*var_id);
            let result =
                compile_expression(working_set, builder, expr, redirect_modes, in_reg, out_reg);
            builder.stream_input_vars.pop();
            result
        }
        Expr::Collect(var_id, expr) => {
            let store_reg = if let Some(in_reg) = in_reg {
                // Collect, clone, store
                builder.push(Instruction::Collect { src_dst: in_reg }.into_spanned(expr.span))?;
                builder.clone_reg(in_reg, expr.span)?
            } else {
                // Just store nothing in the variable
                builder.literal(Literal::Nothing.into_spanned(expr.span))?
            };
            builder.push(
                Instruction::StoreVariable {
                    var_id: *var_id,
                    src: store_reg,
                }
                .into_spanned(expr.span),
            )?;
            compile_expression(working_set, builder, expr, redirect_modes, in_reg, out_reg)?;
            // Clean it up afterward
            builder.push(Instruction::DropVariable { var_id: *var_id }.into_spanned(expr.span))?;
            Ok(())
        }
        Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            compile_block(
                working_set,
                builder,
                block,
                true,
                redirect_modes,
                in_reg,
                out_reg,
            )
        }
        Expr::Block(block_id) => lit(builder, Literal::Block(*block_id)),
        Expr::Closure(block_id) => lit(builder, Literal::Closure(*block_id)),
        Expr::MatchBlock(_) => Err(unexpected("MatchBlock")), // only for `match` keyword
        Expr::List(items) => {
            // Guess capacity based on items (does not consider spread as more than 1)
            lit(
                builder,
                Literal::List {
                    capacity: items.len(),
                },
            )?;
            for item in items {
                // Compile the expression of the item / spread
                let reg = builder.next_register()?;
                let expr = match item {
                    ListItem::Item(expr) | ListItem::Spread(_, expr) => expr,
                };
                compile_expression(
                    working_set,
                    builder,
                    expr,
                    RedirectModes::value(expr.span),
                    None,
                    reg,
                )?;

                match item {
                    ListItem::Item(_) => {
                        // Add each item using list-push
                        builder.push(
                            Instruction::ListPush {
                                src_dst: out_reg,
                                item: reg,
                            }
                            .into_spanned(expr.span),
                        )?;
                    }
                    ListItem::Spread(spread_span, _) => {
                        // Spread the list using list-spread
                        builder.push(
                            Instruction::ListSpread {
                                src_dst: out_reg,
                                items: reg,
                            }
                            .into_spanned(*spread_span),
                        )?;
                    }
                }
            }
            Ok(())
        }
        Expr::Table(table) => {
            lit(
                builder,
                Literal::List {
                    capacity: table.rows.len(),
                },
            )?;

            // Evaluate the columns
            let column_registers = table
                .columns
                .iter()
                .map(|column| {
                    let reg = builder.next_register()?;
                    compile_expression(
                        working_set,
                        builder,
                        column,
                        RedirectModes::value(column.span),
                        None,
                        reg,
                    )?;
                    Ok(reg)
                })
                .collect::<Result<Vec<RegId>, CompileError>>()?;

            // Build records for each row
            for row in table.rows.iter() {
                let row_reg = builder.next_register()?;
                builder.load_literal(
                    row_reg,
                    Literal::Record {
                        capacity: table.columns.len(),
                    }
                    .into_spanned(expr.span),
                )?;
                for (column_reg, item) in column_registers.iter().zip(row.iter()) {
                    let column_reg = builder.clone_reg(*column_reg, item.span)?;
                    let item_reg = builder.next_register()?;
                    compile_expression(
                        working_set,
                        builder,
                        item,
                        RedirectModes::value(item.span),
                        None,
                        item_reg,
                    )?;
                    builder.push(
                        Instruction::RecordInsert {
                            src_dst: row_reg,
                            key: column_reg,
                            val: item_reg,
                        }
                        .into_spanned(item.span),
                    )?;
                }
                builder.push(
                    Instruction::ListPush {
                        src_dst: out_reg,
                        item: row_reg,
                    }
                    .into_spanned(expr.span),
                )?;
            }

            // Free the column registers, since they aren't needed anymore
            for reg in column_registers {
                builder.drop_reg(reg)?;
            }

            Ok(())
        }
        Expr::Record(items) => {
            lit(
                builder,
                Literal::Record {
                    capacity: items.len(),
                },
            )?;

            for item in items {
                match item {
                    RecordItem::Pair(key, val) => {
                        // Add each item using record-insert
                        let key_reg = builder.next_register()?;
                        let val_reg = builder.next_register()?;
                        compile_expression(
                            working_set,
                            builder,
                            key,
                            RedirectModes::value(key.span),
                            None,
                            key_reg,
                        )?;
                        compile_expression(
                            working_set,
                            builder,
                            val,
                            RedirectModes::value(val.span),
                            None,
                            val_reg,
                        )?;
                        builder.push(
                            Instruction::RecordInsert {
                                src_dst: out_reg,
                                key: key_reg,
                                val: val_reg,
                            }
                            .into_spanned(key.span.merge(val.span)),
                        )?;
                    }
                    RecordItem::Spread(spread_span, expr) => {
                        // Spread the expression using record-spread
                        let reg = builder.next_register()?;
                        compile_expression(
                            working_set,
                            builder,
                            expr,
                            RedirectModes::value(expr.span),
                            None,
                            reg,
                        )?;
                        builder.push(
                            Instruction::RecordSpread {
                                src_dst: out_reg,
                                items: reg,
                            }
                            .into_spanned(spread_span.merge(expr.span)),
                        )?;
                    }
                }
            }
            Ok(())
        }
        Expr::Keyword(kw) => {
            // keyword: just pass through expr, since commands that use it and are not being
            // specially handled already are often just positional anyway
            compile_expression(
                working_set,
                builder,
                &kw.expr,
                redirect_modes,
                in_reg,
                out_reg,
            )
        }
        Expr::ValueWithUnit(value_with_unit) => {
            lit(builder, literal_from_value_with_unit(value_with_unit)?)
        }
        Expr::DateTime(dt) => lit(builder, Literal::Date(Box::new(*dt))),
        Expr::Filepath(path, no_expand) => {
            let val = builder.data(path)?;
            lit(
                builder,
                Literal::Filepath {
                    val,
                    no_expand: *no_expand,
                },
            )
        }
        Expr::Directory(path, no_expand) => {
            let val = builder.data(path)?;
            lit(
                builder,
                Literal::Directory {
                    val,
                    no_expand: *no_expand,
                },
            )
        }
        Expr::GlobPattern(path, no_expand) => {
            let val = builder.data(path)?;
            lit(
                builder,
                Literal::GlobPattern {
                    val,
                    no_expand: *no_expand,
                },
            )
        }
        Expr::String(s) => {
            let data_slice = builder.data(s)?;
            lit(builder, Literal::String(data_slice))
        }
        Expr::RawString(rs) => {
            let data_slice = builder.data(rs)?;
            lit(builder, Literal::RawString(data_slice))
        }
        Expr::CellPath(path) => lit(builder, Literal::CellPath(Box::new(path.clone()))),
        Expr::FullCellPath(full_cell_path) => {
            if matches!(full_cell_path.head.expr, Expr::Var(ENV_VARIABLE_ID)) {
                compile_load_env(builder, expr.span, &full_cell_path.tail, out_reg)
            } else {
                compile_expression(
                    working_set,
                    builder,
                    &full_cell_path.head,
                    // Only capture the output if there is a tail. This was a bit of a headscratcher
                    // as the parser emits a FullCellPath with no tail for subexpressions in
                    // general, which shouldn't be captured any differently than they otherwise
                    // would be.
                    if !full_cell_path.tail.is_empty() {
                        RedirectModes::value(expr.span)
                    } else {
                        redirect_modes
                    },
                    in_reg,
                    out_reg,
                )?;
                // Only do the follow if this is actually needed
                if !full_cell_path.tail.is_empty() {
                    let cell_path_reg = builder.literal(
                        Literal::CellPath(Box::new(CellPath {
                            members: full_cell_path.tail.clone(),
                        }))
                        .into_spanned(expr.span),
                    )?;
                    builder.push(
                        Instruction::FollowCellPath {
                            src_dst: out_reg,
                            path: cell_path_reg,
                        }
                        .into_spanned(expr.span),
                    )?;
                }
                Ok(())
            }
        }
        Expr::ImportPattern(_) => Err(unexpected("ImportPattern")),
        Expr::Overlay(_) => Err(unexpected("Overlay")),
        Expr::Signature(_) => ignore(builder), // no effect
        Expr::StringInterpolation(exprs) | Expr::GlobInterpolation(exprs, _) => {
            let mut exprs_iter = exprs.iter().peekable();

            if exprs_iter
                .peek()
                .is_some_and(|e| matches!(e.expr, Expr::String(..) | Expr::RawString(..)))
            {
                // If the first expression is a string or raw string literal, just take it and build
                // from that
                compile_expression(
                    working_set,
                    builder,
                    exprs_iter.next().expect("peek() was Some"),
                    RedirectModes::value(expr.span),
                    None,
                    out_reg,
                )?;
            } else {
                // Start with an empty string
                lit(builder, Literal::String(DataSlice::empty()))?;
            }

            // Compile each expression and append to out_reg
            for expr in exprs_iter {
                let scratch_reg = builder.next_register()?;
                compile_expression(
                    working_set,
                    builder,
                    expr,
                    RedirectModes::value(expr.span),
                    None,
                    scratch_reg,
                )?;
                builder.push(
                    Instruction::StringAppend {
                        src_dst: out_reg,
                        val: scratch_reg,
                    }
                    .into_spanned(expr.span),
                )?;
            }

            // If it's a glob interpolation, change it to a glob
            if let Expr::GlobInterpolation(_, no_expand) = expr.expr {
                builder.push(
                    Instruction::GlobFrom {
                        src_dst: out_reg,
                        no_expand,
                    }
                    .into_spanned(expr.span),
                )?;
            }

            Ok(())
        }
        Expr::Nothing => lit(builder, Literal::Nothing),
        Expr::Garbage => Err(CompileError::Garbage { span: expr.span }),
    }
}

/// Whether the `$in` that a `Collect` binds to `var_id` can take the input without collecting it.
///
/// That holds when `expr` reads it exactly once, and that read is the input of everything else
/// (see [`starts_with_read`]). The first pipeline is the only one that gets the block's input, so
/// a `$in` anywhere else reads it a second time.
fn reads_input_once_at_head(
    working_set: &StateWorkingSet,
    var_id: VarId,
    expr: &Expression,
) -> bool {
    if !starts_with_read(working_set, var_id, expr) {
        return false;
    }

    let is_read = |expr: &Expression| matches!(expr.expr, Expr::Var(id) if id == var_id);
    // Count the reads, including those in nested blocks and closures, which would capture it, and
    // stop at the second one. The head read above is one of them.
    let reads = Cell::new(0);
    let second_read = expr.find_map(working_set, &|expr| {
        if is_read(expr) {
            reads.set(reads.get() + 1);
            if reads.get() > 1 {
                return ControlFlow::Break(Some(()));
            }
        }
        ControlFlow::Continue(())
    });
    second_read.is_none()
}

/// Whether `expr` starts with a read of `var_id` that gets `expr`'s input: `expr` is the variable
/// itself (a bare `$in`, also mid-pipeline), or a block or parenthesized subexpression whose first
/// pipeline starts with such a read, with no redirection of its own. `($in | first 2)` counts, as
/// the parentheses pass their input to their first pipeline.
fn starts_with_read(working_set: &StateWorkingSet, var_id: VarId, expr: &Expression) -> bool {
    match &expr.expr {
        Expr::Var(id) => *id == var_id,
        // The parser gives `$in` and `(...)` on their own as cell paths with no members.
        Expr::FullCellPath(path) => {
            path.tail.is_empty() && starts_with_read(working_set, var_id, &path.head)
        }
        Expr::Subexpression(block_id) => working_set
            .get_block(*block_id)
            .pipelines
            .first()
            .and_then(|pipeline| pipeline.elements.first())
            .is_some_and(|head| {
                head.redirection.is_none() && starts_with_read(working_set, var_id, &head.expr)
            }),
        _ => false,
    }
}

fn literal_from_value_with_unit(value_with_unit: &ValueWithUnit) -> Result<Literal, CompileError> {
    let Expr::Int(int_value) = value_with_unit.expr.expr else {
        return Err(CompileError::UnexpectedExpression {
            expr_name: format!("{:?}", value_with_unit.expr),
            span: value_with_unit.expr.span,
        });
    };

    match value_with_unit
        .unit
        .item
        .build_value(int_value, value_with_unit.expr.span)
        .map_err(|err| CompileError::InvalidLiteral {
            msg: err.to_string(),
            span: value_with_unit.expr.span,
        })? {
        Value::Filesize { val, .. } => Ok(Literal::Filesize(val)),
        Value::Duration { val, .. } => Ok(Literal::Duration(val)),
        other => Err(CompileError::InvalidLiteral {
            msg: format!("bad value returned by Unit::build_value(): {other:?}"),
            span: value_with_unit.unit.span,
        }),
    }
}
