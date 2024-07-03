use super::{
    compile_binary_op, compile_block, compile_call, compile_external_call, compile_load_env,
    BlockBuilder, CompileError, RedirectModes,
};

use nu_protocol::{
    ast::{CellPath, Expr, Expression, ListItem, RecordItem},
    engine::StateWorkingSet,
    ir::{DataSlice, Instruction, Literal},
    IntoSpanned, RegId, ENV_VARIABLE_ID,
};

pub(crate) fn compile_expression(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    expr: &Expression,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    let drop_input = |builder: &mut BlockBuilder| {
        if let Some(in_reg) = in_reg {
            if in_reg != out_reg {
                builder.drop_reg(in_reg)?;
            }
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

    let todo = |msg: &str| CompileError::Todo {
        msg: msg.into(),
        span: Some(expr.span),
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
                        redirect_modes.with_capture_out(part_expr.span),
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
        Expr::Var(var_id) => {
            drop_input(builder)?;
            builder.push(
                Instruction::LoadVariable {
                    dst: out_reg,
                    var_id: *var_id,
                }
                .into_spanned(expr.span),
            )?;
            Ok(())
        }
        Expr::VarDecl(_) => Err(CompileError::UnexpectedExpression {
            expr_name: "VarDecl".into(),
            span: expr.span,
        }),
        Expr::Call(call) => {
            move_in_reg_to_out_reg(builder)?;

            compile_call(working_set, builder, &call, redirect_modes, out_reg)
        }
        Expr::ExternalCall(head, args) => {
            move_in_reg_to_out_reg(builder)?;

            compile_external_call(working_set, builder, head, args, redirect_modes, out_reg)
        }
        Expr::Operator(_) => Err(CompileError::UnexpectedExpression {
            expr_name: "Operator".into(),
            span: expr.span,
        }),
        Expr::RowCondition(_) => Err(todo("RowCondition")),
        Expr::UnaryNot(subexpr) => {
            drop_input(builder)?;
            compile_expression(
                working_set,
                builder,
                subexpr,
                redirect_modes.with_capture_out(subexpr.span),
                None,
                out_reg,
            )?;
            builder.push(Instruction::Not { src_dst: out_reg }.into_spanned(expr.span))?;
            Ok(())
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            if let Expr::Operator(ref operator) = op.expr {
                drop_input(builder)?;
                compile_binary_op(
                    working_set,
                    builder,
                    &lhs,
                    operator.clone().into_spanned(op.span),
                    &rhs,
                    out_reg,
                )
            } else {
                Err(CompileError::UnsupportedOperatorExpression { span: op.span })
            }
        }
        Expr::Subexpression(block_id) => {
            let block = working_set.get_block(*block_id);
            compile_block(
                working_set,
                builder,
                &block,
                redirect_modes,
                in_reg,
                out_reg,
            )
        }
        Expr::Block(block_id) => lit(builder, Literal::Block(*block_id)),
        Expr::Closure(block_id) => lit(builder, Literal::Closure(*block_id)),
        Expr::MatchBlock(_) => Err(todo("MatchBlock")),
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
                    redirect_modes.with_capture_out(expr.span),
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
                        redirect_modes.with_capture_out(column.span),
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
                        redirect_modes.with_capture_out(item.span),
                        None,
                        item_reg,
                    )?;
                    builder.push(
                        Instruction::RecordInsert {
                            src_dst: out_reg,
                            key: column_reg,
                            val: item_reg,
                        }
                        .into_spanned(item.span),
                    )?;
                }
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
                            redirect_modes.with_capture_out(key.span),
                            None,
                            key_reg,
                        )?;
                        compile_expression(
                            working_set,
                            builder,
                            val,
                            redirect_modes.with_capture_out(val.span),
                            None,
                            val_reg,
                        )?;
                        builder.push(
                            Instruction::RecordInsert {
                                src_dst: out_reg,
                                key: key_reg,
                                val: val_reg,
                            }
                            .into_spanned(expr.span),
                        )?;
                    }
                    RecordItem::Spread(spread_span, expr) => {
                        // Spread the expression using record-spread
                        let reg = builder.next_register()?;
                        compile_expression(
                            working_set,
                            builder,
                            expr,
                            redirect_modes.with_capture_out(expr.span),
                            None,
                            reg,
                        )?;
                        builder.push(
                            Instruction::RecordSpread {
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
        Expr::Keyword(_) => Err(CompileError::UnexpectedExpression {
            expr_name: "Keyword".into(),
            span: expr.span,
        }),
        Expr::ValueWithUnit(_) => Err(todo("ValueWithUnit")),
        Expr::DateTime(_) => Err(todo("DateTime")),
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
                    RedirectModes::capture_out(expr.span),
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
        Expr::ImportPattern(_) => Err(todo("ImportPattern")),
        Expr::Overlay(_) => Err(todo("Overlay")),
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
                    redirect_modes.with_capture_out(expr.span),
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
                    redirect_modes.with_capture_out(expr.span),
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
