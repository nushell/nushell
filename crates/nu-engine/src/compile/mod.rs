use nu_protocol::{
    ast::{
        Argument, Assignment, Block, Call, CellPath, Expr, Expression, ExternalArgument, ListItem,
        Math, Operator, PathMember, Pipeline, PipelineRedirection, RecordItem, RedirectionSource,
        RedirectionTarget,
    },
    engine::StateWorkingSet,
    ir::{DataSlice, Instruction, IrBlock, Literal, RedirectMode},
    IntoSpanned, OutDest, RegId, ShellError, Span, Spanned, ENV_VARIABLE_ID,
};

const BLOCK_INPUT: RegId = RegId(0);

/// Compile Nushell pipeline abstract syntax tree (AST) to internal representation (IR) instructions
/// for evaluation.
pub fn compile(working_set: &StateWorkingSet, block: &Block) -> Result<IrBlock, CompileError> {
    let mut builder = BlockBuilder::new();

    compile_block(
        working_set,
        &mut builder,
        block,
        RedirectModes::default(),
        Some(BLOCK_INPUT),
        BLOCK_INPUT,
    )?;

    // A complete block has to end with a `return`
    builder.push(
        Instruction::Return { src: BLOCK_INPUT }
            .into_spanned(block.span.unwrap_or(Span::unknown())),
    )?;

    Ok(builder.finish())
}

#[derive(Default, Clone)]
struct RedirectModes {
    out: Option<Spanned<RedirectMode>>,
    err: Option<Spanned<RedirectMode>>,
}

impl RedirectModes {
    fn capture_out(span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Capture.into_spanned(span)),
            err: None,
        }
    }

    fn with_pipe_out(&self, span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Pipe.into_spanned(span)),
            err: self.err.clone(),
        }
    }

    fn with_capture_out(&self, span: Span) -> Self {
        RedirectModes {
            out: Some(RedirectMode::Capture.into_spanned(span)),
            err: self.err.clone(),
        }
    }
}

fn compile_block(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    block: &Block,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    let span = block.span.unwrap_or(Span::unknown());
    let mut redirect_modes = Some(redirect_modes);
    if !block.pipelines.is_empty() {
        let last_index = block.pipelines.len() - 1;
        for (index, pipeline) in block.pipelines.iter().enumerate() {
            compile_pipeline(
                working_set,
                builder,
                pipeline,
                span,
                // the redirect mode only applies to the last pipeline.
                if index == last_index {
                    redirect_modes
                        .take()
                        .expect("should only take redirect_modes once")
                } else {
                    RedirectModes::default()
                },
                // input is only passed to the first pipeline.
                if index == 0 { in_reg } else { None },
                out_reg,
            )?;

            if index != last_index {
                // Explicitly drain the out reg after each non-final pipeline, because that's how
                // the semicolon functions.
                builder.push(Instruction::Drain { src: out_reg }.into_spanned(span))?;
            }
        }
        Ok(())
    } else if in_reg.is_none() {
        builder.load_empty(out_reg)
    } else {
        Ok(())
    }
}

fn compile_pipeline(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    pipeline: &Pipeline,
    fallback_span: Span,
    redirect_modes: RedirectModes,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    let mut iter = pipeline.elements.iter().peekable();
    let mut in_reg = in_reg;
    let mut redirect_modes = Some(redirect_modes);
    while let Some(element) = iter.next() {
        let span = element.pipe.unwrap_or(fallback_span);

        // We have to get the redirection mode from either the explicit redirection in the pipeline
        // element, or from the next expression if it's specified there. If this is the last
        // element, then it's from whatever is passed in as the mode to use.

        let next_redirect_modes = if let Some(next_element) = iter.peek() {
            // If there's a next element we always pipe out
            redirect_modes_of_expression(working_set, &next_element.expr, span)?
                .with_pipe_out(next_element.pipe.unwrap_or(next_element.expr.span))
        } else {
            redirect_modes
                .take()
                .expect("should only take redirect_modes once")
        };

        let spec_redirect_modes = match &element.redirection {
            Some(PipelineRedirection::Single { source, target }) => {
                let mode = redirection_target_to_mode(working_set, builder, target, false)?;
                match source {
                    RedirectionSource::Stdout => RedirectModes {
                        out: Some(mode),
                        err: None,
                    },
                    RedirectionSource::Stderr => RedirectModes {
                        out: None,
                        err: Some(mode),
                    },
                    RedirectionSource::StdoutAndStderr => RedirectModes {
                        out: Some(mode),
                        err: Some(mode),
                    },
                }
            }
            Some(PipelineRedirection::Separate { out, err }) => {
                let out = redirection_target_to_mode(working_set, builder, out, true)?;
                let err = redirection_target_to_mode(working_set, builder, err, true)?;
                RedirectModes {
                    out: Some(out),
                    err: Some(err),
                }
            }
            None => RedirectModes {
                out: None,
                err: None,
            },
        };

        let out_mode = spec_redirect_modes.out.or(next_redirect_modes.out);
        let err_mode = spec_redirect_modes.err.or(next_redirect_modes.err);

        compile_expression(
            working_set,
            builder,
            &element.expr,
            RedirectModes {
                out: out_mode,
                err: err_mode,
            },
            in_reg,
            out_reg,
        )?;

        // The next pipeline element takes input from this output
        in_reg = Some(out_reg);
    }
    Ok(())
}

fn redirection_target_to_mode(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    target: &RedirectionTarget,
    separate: bool,
) -> Result<Spanned<RedirectMode>, CompileError> {
    Ok(match target {
        RedirectionTarget::File {
            expr,
            append,
            span: redir_span,
        } => {
            let path_reg = builder.next_register()?;
            compile_expression(
                working_set,
                builder,
                expr,
                RedirectModes::capture_out(*redir_span),
                None,
                path_reg,
            )?;
            RedirectMode::File {
                path: path_reg,
                append: *append,
            }
            .into_spanned(*redir_span)
        }
        RedirectionTarget::Pipe { span } => (if separate {
            RedirectMode::Capture
        } else {
            RedirectMode::Pipe
        })
        .into_spanned(*span),
    })
}

fn redirect_modes_of_expression(
    working_set: &StateWorkingSet,
    expression: &Expression,
    redir_span: Span,
) -> Result<RedirectModes, CompileError> {
    let (out, err) = expression.expr.pipe_redirection(&working_set);
    Ok(RedirectModes {
        out: out
            .map(|out| out_dest_to_redirect_mode(out))
            .transpose()?
            .map(|mode| mode.into_spanned(redir_span)),
        err: err
            .map(|err| out_dest_to_redirect_mode(err))
            .transpose()?
            .map(|mode| mode.into_spanned(redir_span)),
    })
}

fn out_dest_to_redirect_mode(out_dest: OutDest) -> Result<RedirectMode, CompileError> {
    match out_dest {
        OutDest::Pipe => Ok(RedirectMode::Pipe),
        OutDest::Capture => Ok(RedirectMode::Capture),
        OutDest::Null => Ok(RedirectMode::Null),
        OutDest::Inherit => Ok(RedirectMode::Inherit),
        OutDest::File(_) => Err(CompileError::InvalidRedirectMode),
    }
}

fn compile_expression(
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
        Expr::VarDecl(_) => Err(CompileError::Todo("VarDecl")),
        Expr::Call(call) => {
            move_in_reg_to_out_reg(builder)?;

            compile_call(working_set, builder, &call, redirect_modes, out_reg)
        }
        Expr::ExternalCall(head, args) => {
            move_in_reg_to_out_reg(builder)?;

            compile_external_call(working_set, builder, head, args, redirect_modes, out_reg)
        }
        Expr::Operator(_) => Err(CompileError::Todo("Operator")),
        Expr::RowCondition(_) => Err(CompileError::Todo("RowCondition")),
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
                Err(CompileError::UnsupportedOperatorExpression)
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
        Expr::MatchBlock(_) => Err(CompileError::Todo("MatchBlock")),
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
        Expr::Keyword(_) => Err(CompileError::Todo("Keyword")),
        Expr::ValueWithUnit(_) => Err(CompileError::Todo("ValueWithUnit")),
        Expr::DateTime(_) => Err(CompileError::Todo("DateTime")),
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
        Expr::ImportPattern(_) => Err(CompileError::Todo("ImportPattern")),
        Expr::Overlay(_) => Err(CompileError::Todo("Overlay")),
        Expr::Signature(_) => ignore(builder), // no effect
        Expr::StringInterpolation(_) => Err(CompileError::Todo("StringInterpolation")),
        Expr::GlobInterpolation(_, _) => Err(CompileError::Todo("GlobInterpolation")),
        Expr::Nothing => lit(builder, Literal::Nothing),
        Expr::Garbage => Err(CompileError::Garbage),
    }
}

fn compile_call(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // First, try to figure out if this is a keyword call like `if`, and handle those specially
    let decl = working_set.get_decl(call.decl_id);
    if decl.is_keyword() {
        match decl.name() {
            "if" => {
                return compile_if(working_set, builder, call, redirect_modes, io_reg);
            }
            "let" | "mut" => {
                return compile_let(working_set, builder, call, redirect_modes, io_reg);
            }
            _ => (),
        }
    }

    // It's important that we evaluate the args first before trying to set up the argument
    // state for the call.
    //
    // We could technically compile anything that isn't another call safely without worrying about
    // the argument state, but we'd have to check all of that first and it just isn't really worth
    // it.
    enum CompiledArg<'a> {
        Positional(RegId, Span),
        Named(&'a str, Option<RegId>, Span),
        Spread(RegId, Span),
    }

    let mut compiled_args = vec![];

    for arg in &call.arguments {
        let arg_reg = arg
            .expr()
            .map(|expr| {
                let arg_reg = builder.next_register()?;

                compile_expression(
                    working_set,
                    builder,
                    expr,
                    RedirectModes::capture_out(arg.span()),
                    None,
                    arg_reg,
                )?;

                Ok(arg_reg)
            })
            .transpose()?;

        match arg {
            Argument::Positional(_) => compiled_args.push(CompiledArg::Positional(
                arg_reg.expect("expr() None in non-Named"),
                arg.span(),
            )),
            Argument::Named((name, _, _)) => {
                compiled_args.push(CompiledArg::Named(name.item.as_str(), arg_reg, arg.span()))
            }
            Argument::Unknown(_) => return Err(CompileError::Garbage),
            Argument::Spread(_) => compiled_args.push(CompiledArg::Spread(
                arg_reg.expect("expr() None in non-Named"),
                arg.span(),
            )),
        }
    }

    // Now that the args are all compiled, set up the call state (argument stack and redirections)
    for arg in compiled_args {
        match arg {
            CompiledArg::Positional(reg, span) => {
                builder.push(Instruction::PushPositional { src: reg }.into_spanned(span))?;
            }
            CompiledArg::Named(name, Some(reg), span) => {
                let name = builder.data(name)?;
                builder.push(Instruction::PushNamed { name, src: reg }.into_spanned(span))?;
            }
            CompiledArg::Named(name, None, span) => {
                let name = builder.data(name)?;
                builder.push(Instruction::PushFlag { name }.into_spanned(span))?;
            }
            CompiledArg::Spread(reg, span) => {
                builder.push(Instruction::AppendRest { src: reg }.into_spanned(span))?;
            }
        }
    }

    if let Some(mode) = redirect_modes.out {
        builder.push(mode.map(|mode| Instruction::RedirectOut { mode }))?;
    }

    if let Some(mode) = redirect_modes.err {
        builder.push(mode.map(|mode| Instruction::RedirectErr { mode }))?;
    }

    // The state is set up, so we can do the call into io_reg
    builder.push(
        Instruction::Call {
            decl_id: call.decl_id,
            src_dst: io_reg,
        }
        .into_spanned(call.head),
    )?;

    Ok(())
}

fn compile_external_call(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    head: &Expression,
    args: &[ExternalArgument],
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pass everything to run-external
    let run_external_id = working_set
        .find_decl(b"run-external")
        .ok_or_else(|| CompileError::RunExternalNotFound)?;

    let mut call = Call::new(head.span);
    call.decl_id = run_external_id;

    call.arguments.push(Argument::Positional(head.clone()));

    for arg in args {
        match arg {
            ExternalArgument::Regular(expr) => {
                call.arguments.push(Argument::Positional(expr.clone()));
            }
            ExternalArgument::Spread(expr) => {
                call.arguments.push(Argument::Spread(expr.clone()));
            }
        }
    }

    compile_call(working_set, builder, &call, redirect_modes, io_reg)
}

/// Compile a call to `if` as a branch-if
fn compile_if(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    let invalid = || CompileError::InvalidKeywordCall("if", call.head);

    let condition = call.positional_nth(0).ok_or_else(invalid)?;
    let true_block_arg = call.positional_nth(1).ok_or_else(invalid)?;
    let else_arg = call.positional_nth(2);

    let true_block_id = true_block_arg.as_block().ok_or_else(invalid)?;
    let true_block = working_set.get_block(true_block_id);

    let not_condition_reg = {
        // Compile the condition first
        let condition_reg = builder.next_register()?;
        compile_expression(
            working_set,
            builder,
            condition,
            redirect_modes.with_capture_out(condition.span),
            None,
            condition_reg,
        )?;

        // Negate the condition - we basically only want to jump if the condition is false
        builder.push(
            Instruction::Not {
                src_dst: condition_reg,
            }
            .into_spanned(call.head),
        )?;

        condition_reg
    };

    // Set up a branch if the condition is false. Will go back and fix this to the right offset
    let index_of_branch_if = builder.push(
        Instruction::BranchIf {
            cond: not_condition_reg,
            index: usize::MAX,
        }
        .into_spanned(call.head),
    )?;

    // Compile the true case
    compile_block(
        working_set,
        builder,
        true_block,
        redirect_modes.clone(),
        Some(io_reg),
        io_reg,
    )?;

    // Add a jump over the false case
    let index_of_jump =
        builder.push(Instruction::Jump { index: usize::MAX }.into_spanned(call.head))?;

    // Change the branch-if target to after the jump
    builder.set_branch_target(index_of_branch_if, index_of_jump + 1)?;

    // On the else side now, assert that io_reg is still valid
    builder.mark_register(io_reg)?;

    if let Some(else_arg) = else_arg {
        let Expression {
            expr: Expr::Keyword(else_keyword),
            ..
        } = else_arg
        else {
            return Err(invalid());
        };

        if else_keyword.keyword.as_ref() != b"else" {
            return Err(invalid());
        }

        let else_expr = &else_keyword.expr;

        match &else_expr.expr {
            Expr::Block(block_id) => {
                let false_block = working_set.get_block(*block_id);
                compile_block(
                    working_set,
                    builder,
                    false_block,
                    redirect_modes,
                    Some(io_reg),
                    io_reg,
                )?;
            }
            _ => {
                // The else case supports bare expressions too, not only blocks
                compile_expression(
                    working_set,
                    builder,
                    else_expr,
                    redirect_modes,
                    Some(io_reg),
                    io_reg,
                )?;
            }
        }
    } else {
        // We don't have an else expression/block, so just set io_reg = Empty
        builder.load_empty(io_reg)?;
    }

    // Change the jump target to the next index (out of the if-else)
    builder.set_branch_target(index_of_jump, builder.next_instruction_index())?;

    Ok(())
}

/// Compile a call to `let` or `mut` (just do store-variable)
fn compile_let(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    let invalid = || CompileError::InvalidKeywordCall("let", call.head);

    let var_decl_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_arg = call.positional_nth(1).ok_or_else(invalid)?;

    let var_id = var_decl_arg.as_var().ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    compile_block(
        working_set,
        builder,
        block,
        redirect_modes.with_capture_out(call.head),
        None,
        io_reg,
    )?;

    builder.push(
        Instruction::StoreVariable {
            var_id,
            src: io_reg,
        }
        .into_spanned(call.head),
    )?;

    // Don't forget to set io_reg to Empty afterward, as that's the result of an assignment
    builder.load_empty(io_reg)?;

    Ok(())
}

fn compile_binary_op(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    lhs: &Expression,
    op: Spanned<Operator>,
    rhs: &Expression,
    out_reg: RegId,
) -> Result<(), CompileError> {
    if let Operator::Assignment(assign_op) = op.item {
        if let Some(decomposed_op) = decompose_assignment(assign_op) {
            // Compiling an assignment that uses a binary op with the existing value
            compile_binary_op(
                working_set,
                builder,
                lhs,
                decomposed_op.into_spanned(op.span),
                rhs,
                out_reg,
            )?;
        } else {
            // Compiling a plain assignment, where the current left-hand side value doesn't matter
            compile_expression(
                working_set,
                builder,
                rhs,
                RedirectModes::capture_out(rhs.span),
                None,
                out_reg,
            )?;
        }

        compile_assignment(working_set, builder, lhs, op.span, out_reg)?;

        // Load out_reg with Nothing, as that's the result of an assignment
        builder.load_literal(out_reg, Literal::Nothing.into_spanned(op.span))
    } else {
        // Not an assignment: just do the binary op
        let lhs_reg = out_reg;
        let rhs_reg = builder.next_register()?;

        compile_expression(
            working_set,
            builder,
            lhs,
            RedirectModes::capture_out(lhs.span),
            None,
            lhs_reg,
        )?;
        compile_expression(
            working_set,
            builder,
            rhs,
            RedirectModes::capture_out(rhs.span),
            None,
            rhs_reg,
        )?;

        builder.push(
            Instruction::BinaryOp {
                lhs_dst: lhs_reg,
                op: op.item,
                rhs: rhs_reg,
            }
            .into_spanned(op.span),
        )?;

        if lhs_reg != out_reg {
            builder.push(
                Instruction::Move {
                    dst: out_reg,
                    src: lhs_reg,
                }
                .into_spanned(op.span),
            )?;
        }

        Ok(())
    }
}

/// The equivalent plain operator to use for an assignment, if any
fn decompose_assignment(assignment: Assignment) -> Option<Operator> {
    match assignment {
        Assignment::Assign => None,
        Assignment::PlusAssign => Some(Operator::Math(Math::Plus)),
        Assignment::AppendAssign => Some(Operator::Math(Math::Append)),
        Assignment::MinusAssign => Some(Operator::Math(Math::Minus)),
        Assignment::MultiplyAssign => Some(Operator::Math(Math::Multiply)),
        Assignment::DivideAssign => Some(Operator::Math(Math::Divide)),
    }
}

/// Compile assignment of the value in a register to a left-hand expression
fn compile_assignment(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    lhs: &Expression,
    assignment_span: Span,
    rhs_reg: RegId,
) -> Result<(), CompileError> {
    match lhs.expr {
        Expr::Var(var_id) => {
            // Double check that the variable is supposed to be mutable
            if !working_set.get_variable(var_id).mutable {
                return Err(CompileError::ModifyImmutableVariable(lhs.span));
            }

            builder.push(
                Instruction::StoreVariable {
                    var_id,
                    src: rhs_reg,
                }
                .into_spanned(assignment_span),
            )?;
            Ok(())
        }
        Expr::FullCellPath(ref path) => match (&path.head, &path.tail) {
            (
                Expression {
                    expr: Expr::Var(var_id),
                    ..
                },
                _,
            ) if *var_id == ENV_VARIABLE_ID => {
                // This will be an assignment to an environment variable.
                let Some(PathMember::String {
                    val: key, optional, ..
                }) = path.tail.first()
                else {
                    return Err(CompileError::InvalidLhsForAssignment(lhs.span));
                };

                let key_data = builder.data(key)?;

                let val_reg = if path.tail.len() > 1 {
                    // Get the current value of the head and first tail of the path, from env
                    let head_reg = builder.next_register()?;

                    // We could use compile_load_env, but this shares the key data...
                    if *optional {
                        builder.push(
                            Instruction::LoadEnvOpt {
                                dst: head_reg,
                                key: key_data,
                            }
                            .into_spanned(lhs.span),
                        )?;
                    } else {
                        builder.push(
                            Instruction::LoadEnv {
                                dst: head_reg,
                                key: key_data,
                            }
                            .into_spanned(lhs.span),
                        )?;
                    }

                    // Do the upsert on the current value to incorporate rhs
                    compile_upsert_cell_path(
                        builder,
                        (&path.tail[1..]).into_spanned(lhs.span),
                        head_reg,
                        rhs_reg,
                        assignment_span,
                    )?;

                    head_reg
                } else {
                    // Path has only one tail, so we don't need the current value to do an upsert,
                    // just set it directly to rhs
                    rhs_reg
                };

                // Finally, store the modified env variable
                builder.push(
                    Instruction::StoreEnv {
                        key: key_data,
                        src: val_reg,
                    }
                    .into_spanned(assignment_span),
                )?;
                Ok(())
            }
            (_, tail) if tail.is_empty() => {
                // If the path tail is empty, we can really just treat this as if it were an
                // assignment to the head
                compile_assignment(working_set, builder, &path.head, assignment_span, rhs_reg)
            }
            _ => {
                // Just a normal assignment to some path
                let head_reg = builder.next_register()?;

                // Compile getting current value of the head expression
                compile_expression(
                    working_set,
                    builder,
                    &path.head,
                    RedirectModes::capture_out(path.head.span),
                    None,
                    head_reg,
                )?;

                // Upsert the tail of the path into the old value of the head expression
                compile_upsert_cell_path(
                    builder,
                    path.tail.as_slice().into_spanned(lhs.span),
                    head_reg,
                    rhs_reg,
                    assignment_span,
                )?;

                // Now compile the assignment of the updated value to the head
                compile_assignment(working_set, builder, &path.head, assignment_span, head_reg)
            }
        },
        Expr::Garbage => Err(CompileError::Garbage),
        _ => Err(CompileError::InvalidLhsForAssignment(lhs.span)),
    }
}

/// Compile an upsert-cell-path instruction, with known literal members
fn compile_upsert_cell_path(
    builder: &mut BlockBuilder,
    members: Spanned<&[PathMember]>,
    src_dst: RegId,
    new_value: RegId,
    span: Span,
) -> Result<(), CompileError> {
    let path_reg = builder.literal(
        Literal::CellPath(
            CellPath {
                members: members.item.to_vec(),
            }
            .into(),
        )
        .into_spanned(members.span),
    )?;
    builder.push(
        Instruction::UpsertCellPath {
            src_dst,
            path: path_reg,
            new_value,
        }
        .into_spanned(span),
    )?;
    Ok(())
}

/// Compile the correct sequence to get an environment variable + follow a path on it
fn compile_load_env(
    builder: &mut BlockBuilder,
    span: Span,
    path: &[PathMember],
    out_reg: RegId,
) -> Result<(), CompileError> {
    if path.is_empty() {
        builder.push(
            Instruction::LoadVariable {
                dst: out_reg,
                var_id: ENV_VARIABLE_ID,
            }
            .into_spanned(span),
        )?;
    } else {
        let (key, optional) = match &path[0] {
            PathMember::String { val, optional, .. } => (builder.data(val)?, *optional),
            PathMember::Int { span, .. } => return Err(CompileError::AccessEnvByInt(*span)),
        };
        let tail = &path[1..];

        if optional {
            builder.push(Instruction::LoadEnvOpt { dst: out_reg, key }.into_spanned(span))?;
        } else {
            builder.push(Instruction::LoadEnv { dst: out_reg, key }.into_spanned(span))?;
        }

        if !tail.is_empty() {
            let path = builder.literal(
                Literal::CellPath(Box::new(CellPath {
                    members: tail.to_vec(),
                }))
                .into_spanned(span),
            )?;
            builder.push(
                Instruction::FollowCellPath {
                    src_dst: out_reg,
                    path,
                }
                .into_spanned(span),
            )?;
        }
    }
    Ok(())
}

/// An internal compiler error, generally means a Nushell bug rather than an issue with user error
/// since parsing and typechecking has already passed.
#[derive(Debug)]
pub enum CompileError {
    RegisterOverflow,
    RegisterUninitialized(RegId),
    DataOverflow,
    InvalidRedirectMode,
    Garbage,
    UnsupportedOperatorExpression,
    AccessEnvByInt(Span),
    InvalidKeywordCall(&'static str, Span),
    SetBranchTargetOfNonBranchInstruction,
    InstructionIndexOutOfRange(usize),
    RunExternalNotFound,
    InvalidLhsForAssignment(Span),
    ModifyImmutableVariable(Span),
    Todo(&'static str),
}

impl CompileError {
    pub fn message(&self) -> String {
        match self {
            CompileError::RegisterOverflow => format!("register overflow"),
            CompileError::RegisterUninitialized(reg_id) => {
                format!("register {reg_id} is uninitialized when used, possibly reused")
            }
            CompileError::DataOverflow => {
                format!("block contains too much string data: maximum 4 GiB exceeded")
            }
            CompileError::InvalidRedirectMode => {
                "invalid redirect mode: File should not be specified by commands".into()
            }
            CompileError::Garbage => "encountered garbage, likely due to parse error".into(),
            CompileError::UnsupportedOperatorExpression => "unsupported operator expression".into(),
            CompileError::AccessEnvByInt(_) => "attempted access of $env by integer path".into(),
            CompileError::InvalidKeywordCall(kind, _) => format!("invalid `{kind}` keyword cal"),
            CompileError::SetBranchTargetOfNonBranchInstruction => {
                "attempted to set branch target of non-branch instruction".into()
            }
            CompileError::InstructionIndexOutOfRange(index) => {
                format!("instruction index out of range: {index}")
            }
            CompileError::RunExternalNotFound => {
                "run-external is not supported here, so external calls can't be compiled".into()
            }
            CompileError::InvalidLhsForAssignment(_) => {
                "invalid left-hand side for assignment".into()
            }
            CompileError::ModifyImmutableVariable(_) => {
                "attempted to modify immutable variable".into()
            }
            CompileError::Todo(msg) => {
                format!("TODO: {msg}")
            }
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            CompileError::AccessEnvByInt(span)
            | CompileError::InvalidKeywordCall(_, span)
            | CompileError::InvalidLhsForAssignment(span)
            | CompileError::ModifyImmutableVariable(span) => Some(*span),
            _ => None,
        }
    }

    pub fn to_shell_error(self, span: Option<Span>) -> ShellError {
        ShellError::IrCompileError {
            msg: self.message(),
            span: self.span().or(span),
        }
    }
}

/// Builds [`IrBlock`]s progressively by consuming instructions and handles register allocation.
#[derive(Debug)]
struct BlockBuilder {
    instructions: Vec<Instruction>,
    spans: Vec<Span>,
    data: Vec<u8>,
    register_allocation_state: Vec<bool>,
}

impl BlockBuilder {
    /// Starts a new block, with the first register (`%0`) allocated as input.
    fn new() -> Self {
        BlockBuilder {
            instructions: vec![],
            spans: vec![],
            data: vec![],
            register_allocation_state: vec![true],
        }
    }

    /// Get the next unused register for code generation.
    fn next_register(&mut self) -> Result<RegId, CompileError> {
        if let Some(index) = self
            .register_allocation_state
            .iter_mut()
            .position(|is_allocated| {
                if !*is_allocated {
                    *is_allocated = true;
                    true
                } else {
                    false
                }
            })
        {
            Ok(RegId(index as u32))
        } else if self.register_allocation_state.len() < (u32::MAX as usize - 2) {
            let reg_id = RegId(self.register_allocation_state.len() as u32);
            self.register_allocation_state.push(true);
            Ok(reg_id)
        } else {
            Err(CompileError::RegisterOverflow)
        }
    }

    /// Mark a register as initialized.
    fn mark_register(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        if let Some(is_allocated) = self.register_allocation_state.get_mut(reg_id.0 as usize) {
            *is_allocated = true;
            Ok(())
        } else {
            Err(CompileError::RegisterOverflow)
        }
    }

    /// Mark a register as empty, so that it can be used again by something else.
    fn free_register(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        let index = reg_id.0 as usize;

        if self
            .register_allocation_state
            .get(index)
            .is_some_and(|is_allocated| *is_allocated)
        {
            self.register_allocation_state[index] = false;
            Ok(())
        } else {
            log::warn!("register {reg_id} uninitialized, builder = {self:#?}");
            Err(CompileError::RegisterUninitialized(reg_id))
        }
    }

    /// Insert an instruction into the block, automatically marking any registers populated by
    /// the instruction, and freeing any registers consumed by the instruction.
    ///
    /// Returns the offset of the inserted instruction.
    fn push(&mut self, instruction: Spanned<Instruction>) -> Result<usize, CompileError> {
        match &instruction.item {
            Instruction::LoadLiteral { dst, lit } => {
                self.mark_register(*dst)?;
                // Free any registers on the literal
                match lit {
                    Literal::Range {
                        start,
                        step,
                        end,
                        inclusion: _,
                    } => {
                        self.free_register(*start)?;
                        self.free_register(*step)?;
                        self.free_register(*end)?;
                    }
                    Literal::Bool(_)
                    | Literal::Int(_)
                    | Literal::Float(_)
                    | Literal::Binary(_)
                    | Literal::Block(_)
                    | Literal::Closure(_)
                    | Literal::List { capacity: _ }
                    | Literal::Record { capacity: _ }
                    | Literal::Filepath {
                        val: _,
                        no_expand: _,
                    }
                    | Literal::Directory {
                        val: _,
                        no_expand: _,
                    }
                    | Literal::GlobPattern {
                        val: _,
                        no_expand: _,
                    }
                    | Literal::String(_)
                    | Literal::RawString(_)
                    | Literal::CellPath(_)
                    | Literal::Nothing => (),
                }
            }
            Instruction::Move { dst, src } => {
                self.free_register(*src)?;
                self.mark_register(*dst)?;
            }
            Instruction::Clone { dst, src: _ } => self.mark_register(*dst)?,
            Instruction::Collect { src_dst: _ } => (),
            Instruction::Drop { src } => self.free_register(*src)?,
            Instruction::Drain { src } => self.free_register(*src)?,
            Instruction::LoadVariable { dst, var_id: _ } => self.mark_register(*dst)?,
            Instruction::StoreVariable { var_id: _, src } => self.free_register(*src)?,
            Instruction::LoadEnv { dst, key: _ } => self.mark_register(*dst)?,
            Instruction::LoadEnvOpt { dst, key: _ } => self.mark_register(*dst)?,
            Instruction::StoreEnv { key: _, src } => self.free_register(*src)?,
            Instruction::PushPositional { src } => self.free_register(*src)?,
            Instruction::AppendRest { src } => self.free_register(*src)?,
            Instruction::PushFlag { name: _ } => (),
            Instruction::PushNamed { name: _, src } => self.free_register(*src)?,
            Instruction::RedirectOut { mode } | Instruction::RedirectErr { mode } => match mode {
                RedirectMode::File { path, .. } => self.free_register(*path)?,
                _ => (),
            },
            Instruction::Call {
                decl_id: _,
                src_dst: _,
            } => (),
            Instruction::ListPush { src_dst: _, item } => self.free_register(*item)?,
            Instruction::ListSpread { src_dst: _, items } => self.free_register(*items)?,
            Instruction::RecordInsert {
                src_dst: _,
                key,
                val,
            } => {
                self.free_register(*key)?;
                self.free_register(*val)?;
            }
            Instruction::RecordSpread { src_dst: _, items } => self.free_register(*items)?,
            Instruction::Not { src_dst: _ } => (),
            Instruction::BinaryOp {
                lhs_dst: _,
                op: _,
                rhs,
            } => self.free_register(*rhs)?,
            Instruction::FollowCellPath { src_dst: _, path } => self.free_register(*path)?,
            Instruction::CloneCellPath { dst, src: _, path } => {
                self.mark_register(*dst)?;
                self.free_register(*path)?;
            }
            Instruction::UpsertCellPath {
                src_dst: _,
                path,
                new_value,
            } => {
                self.free_register(*path)?;
                self.free_register(*new_value)?;
            }
            Instruction::Jump { index: _ } => (),
            Instruction::BranchIf { cond, index: _ } => self.free_register(*cond)?,
            Instruction::Return { src } => self.free_register(*src)?,
        }
        let index = self.next_instruction_index();
        self.instructions.push(instruction.item);
        self.spans.push(instruction.span);
        Ok(index)
    }

    /// Load a register with a literal.
    fn load_literal(
        &mut self,
        reg_id: RegId,
        literal: Spanned<Literal>,
    ) -> Result<(), CompileError> {
        self.push(
            Instruction::LoadLiteral {
                dst: reg_id,
                lit: literal.item,
            }
            .into_spanned(literal.span),
        )?;
        Ok(())
    }

    /// Allocate a new register and load a literal into it.
    fn literal(&mut self, literal: Spanned<Literal>) -> Result<RegId, CompileError> {
        let reg_id = self.next_register()?;
        self.load_literal(reg_id, literal)?;
        Ok(reg_id)
    }

    /// Deallocate a register and set it to `Empty`
    fn drop_reg(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        self.push(Instruction::Drop { src: reg_id }.into_spanned(Span::unknown()))?;
        Ok(())
    }

    /// Set a register to `Empty`, but mark it as in-use, e.g. for input
    fn load_empty(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        self.mark_register(reg_id)?;
        self.drop_reg(reg_id)?;
        self.mark_register(reg_id)
    }

    /// Add data to the `data` array and return a [`DataSlice`] referencing it.
    fn data(&mut self, data: impl AsRef<[u8]>) -> Result<DataSlice, CompileError> {
        let start = self.data.len();
        if start + data.as_ref().len() < u32::MAX as usize {
            let slice = DataSlice {
                start: start as u32,
                len: data.as_ref().len() as u32,
            };
            self.data.extend_from_slice(data.as_ref());
            Ok(slice)
        } else {
            Err(CompileError::DataOverflow)
        }
    }

    /// Clone a register with a `clone` instruction.
    fn clone_reg(&mut self, src: RegId, span: Span) -> Result<RegId, CompileError> {
        let dst = self.next_register()?;
        self.push(Instruction::Clone { dst, src }.into_spanned(span))?;
        Ok(dst)
    }

    /// Modify a `branch-if` or `jump` instruction's branch target `index`
    fn set_branch_target(
        &mut self,
        instruction_index: usize,
        target_index: usize,
    ) -> Result<(), CompileError> {
        match self.instructions.get_mut(instruction_index) {
            Some(Instruction::BranchIf { index, .. }) | Some(Instruction::Jump { index }) => {
                *index = target_index;
                Ok(())
            }
            Some(_) => Err(CompileError::SetBranchTargetOfNonBranchInstruction),
            None => Err(CompileError::InstructionIndexOutOfRange(instruction_index)),
        }
    }

    /// The index that the next instruction [`.push()`]ed will have.
    fn next_instruction_index(&self) -> usize {
        self.instructions.len()
    }

    /// Consume the builder and produce the final [`IrBlock`].
    fn finish(self) -> IrBlock {
        IrBlock {
            instructions: self.instructions,
            spans: self.spans,
            data: self.data.into(),
            register_count: self.register_allocation_state.len(),
        }
    }
}
