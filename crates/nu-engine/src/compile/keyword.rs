use nu_protocol::{
    ast::{Block, Call, Expr, Expression},
    engine::StateWorkingSet,
    ir::Instruction,
    IntoSpanned, RegId, VarId,
};

use super::{compile_block, compile_expression, BlockBuilder, CompileError, RedirectModes};

/// Compile a call to `if` as a branch-if
pub(crate) fn compile_if(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    //        %io_reg <- <condition>
    //        not %io_reg
    //        branch-if %io_reg, FALSE
    // TRUE:  ...<true_block>...
    //        jump END
    // FALSE: ...<else_expr>... OR drop %io_reg
    // END:
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "if".into(),
        span: call.head,
    };

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
    let index_of_branch_if = builder.branch_if_placeholder(not_condition_reg, call.head)?;

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
    let index_of_jump = builder.jump_placeholder(else_arg.map(|e| e.span).unwrap_or(call.head))?;

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

/// Compile a call to `match`
pub(crate) fn compile_match(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    //         %match_reg <- <match_expr>
    //         collect %match_reg
    //         match (pat1), %match_reg, PAT1
    // MATCH2: match (pat2), %match_reg, PAT2
    // FAIL:   drop %io_reg
    //         drop %match_reg
    //         jump END
    // PAT1:   %guard_reg <- <guard_expr>
    //         not %guard_reg
    //         branch-if %guard_reg, MATCH2
    //         drop %match_reg
    //         <...expr...>
    //         jump END
    // PAT2:   drop %match_reg
    //         <...expr...>
    //         jump END
    // END:
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "match".into(),
        span: call.head,
    };

    let match_expr = call.positional_nth(0).ok_or_else(invalid)?;

    let match_block_arg = call.positional_nth(1).ok_or_else(invalid)?;
    let match_block = match_block_arg.as_match_block().ok_or_else(invalid)?;

    let match_reg = builder.next_register()?;

    // Evaluate the match expression (patterns will be checked against this).
    compile_expression(
        working_set,
        builder,
        match_expr,
        redirect_modes.with_capture_out(match_expr.span),
        None,
        match_reg,
    )?;

    // Important to collect it first
    builder.push(Instruction::Collect { src_dst: match_reg }.into_spanned(match_expr.span))?;

    // Generate the `match` instructions. Guards are not used at this stage.
    let match_offset = builder.next_instruction_index();

    for (pattern, _) in match_block {
        builder.push(
            Instruction::Match {
                pattern: Box::new(pattern.pattern.clone()),
                src: match_reg,
                index: usize::MAX, // placeholder
            }
            .into_spanned(pattern.span),
        )?;
    }

    let mut end_jumps = Vec::with_capacity(match_block.len() + 1);

    // Match fall-through to jump to the end, if no match
    builder.load_empty(io_reg)?;
    builder.drop_reg(match_reg)?;
    end_jumps.push(builder.jump_placeholder(call.head)?);

    // Generate each of the match expressions. Handle guards here, if present.
    for (index, (pattern, expr)) in match_block.iter().enumerate() {
        // `io_reg` and `match_reg` are still valid at each of these branch targets
        builder.mark_register(io_reg)?;
        builder.mark_register(match_reg)?;

        // Set the original match instruction target here
        builder.set_branch_target(match_offset + index, builder.next_instruction_index())?;

        // Handle guard, if present
        if let Some(guard) = &pattern.guard {
            let guard_reg = builder.next_register()?;
            compile_expression(
                working_set,
                builder,
                guard,
                redirect_modes.with_capture_out(guard.span),
                None,
                guard_reg,
            )?;
            builder.push(Instruction::Not { src_dst: guard_reg }.into_spanned(guard.span))?;
            // Branch to the next match instruction if the branch fails to match
            builder.push(
                Instruction::BranchIf {
                    cond: guard_reg,
                    index: match_offset + index + 1,
                }
                .into_spanned(
                    // Span the branch with the next pattern, or the head if this is the end
                    match_block
                        .get(index + 1)
                        .map(|b| b.0.span)
                        .unwrap_or(call.head),
                ),
            )?;
        }

        // match_reg no longer needed, successful match
        builder.drop_reg(match_reg)?;

        // Execute match right hand side expression
        if let Some(block_id) = expr.as_block() {
            let block = working_set.get_block(block_id);
            compile_block(
                working_set,
                builder,
                block,
                redirect_modes.clone(),
                Some(io_reg),
                io_reg,
            )?;
        } else {
            compile_expression(
                working_set,
                builder,
                expr,
                redirect_modes.clone(),
                Some(io_reg),
                io_reg,
            )?;
        }

        // Rewrite this jump to the end afterward
        end_jumps.push(builder.jump_placeholder(call.head)?);
    }

    // Rewrite the end jumps to the next instruction
    for index in end_jumps {
        builder.set_branch_target(index, builder.next_instruction_index())?;
    }

    Ok(())
}

/// Compile a call to `let` or `mut` (just do store-variable)
pub(crate) fn compile_let(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // %io_reg <- ...<block>... <- %io_reg
    // store-variable $var, %io_reg
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "let".into(),
        span: call.head,
    };

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
        Some(io_reg),
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

/// Compile a call to `try`, setting an error handler over the evaluated block
pub(crate) fn compile_try(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode (literal block):
    //
    //      on-error-with ERR, %io_reg           // or without
    //      %io_reg <- <...block...> <- %io_reg
    //      pop-error-handler
    //      jump END
    // ERR: store-variable $err_var, %io_reg     // or without
    //      %io_reg <- <...catch block...>       // set to empty if no catch block
    // END:
    //
    // with expression that can't be inlined:
    //
    //      %closure_reg <- <catch_expr>
    //      on-error-with ERR, %io_reg
    //      %io_reg <- <...block...> <- %io_reg
    //      pop-error-handler
    //      jump END
    // ERR: push-positional %closure_reg
    //      push-positional %io_reg
    //      call "do", %io_reg
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "try".into(),
        span: call.head,
    };

    let block_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    let catch_expr = match call.positional_nth(1) {
        Some(kw_expr) => Some(kw_expr.as_keyword().ok_or_else(invalid)?),
        None => None,
    };

    // We have two ways of executing `catch`: if it was provided as a literal, we can inline it.
    // Otherwise, we have to evaluate the expression and keep it as a register, and then call `do`.
    enum CatchType<'a> {
        Block {
            block: &'a Block,
            var_id: Option<VarId>,
        },
        Closure {
            closure_reg: RegId,
        },
    }

    let catch_type = catch_expr
        .map(|catch_expr| match catch_expr.as_block() {
            Some(block_id) => {
                let block = working_set.get_block(block_id);
                let var_id = block.signature.get_positional(0).and_then(|v| v.var_id);
                Ok(CatchType::Block { block, var_id })
            }
            None => {
                // We have to compile the catch_expr and use it as a closure
                let closure_reg = builder.next_register()?;
                compile_expression(
                    working_set,
                    builder,
                    catch_expr,
                    redirect_modes.with_capture_out(catch_expr.span),
                    None,
                    closure_reg,
                )?;
                Ok(CatchType::Closure { closure_reg })
            }
        })
        .transpose()?;

    // Put the error handler placeholder. If the catch argument is a non-block expression or a block
    // that takes an argument, we should capture the error into `io_reg` since we safely don't need
    // that.
    let error_handler_index = if matches!(
        catch_type,
        Some(
            CatchType::Block {
                var_id: Some(_),
                ..
            } | CatchType::Closure { .. }
        )
    ) {
        builder.push(
            Instruction::OnErrorInto {
                index: usize::MAX,
                dst: io_reg,
            }
            .into_spanned(call.head),
        )?
    } else {
        // Otherwise, we don't need the error value.
        builder.push(Instruction::OnError { index: usize::MAX }.into_spanned(call.head))?
    };

    // Compile the block
    compile_block(
        working_set,
        builder,
        block,
        redirect_modes.clone(),
        Some(io_reg),
        io_reg,
    )?;

    // Successful case: pop the error handler
    builder.push(Instruction::PopErrorHandler.into_spanned(call.head))?;

    // Jump over the failure case
    let catch_span = catch_expr.map(|e| e.span).unwrap_or(call.head);
    let jump_index = builder.jump_placeholder(catch_span)?;

    // This is the error handler - go back and set the right branch destination
    builder.set_branch_target(error_handler_index, builder.next_instruction_index())?;

    // Mark out register as likely not clean - state in error handler is not well defined
    builder.mark_register(io_reg)?;

    // Now compile whatever is necessary for the error handler
    match catch_type {
        Some(CatchType::Block { block, var_id }) => {
            if let Some(var_id) = var_id {
                // Error will be in io_reg
                builder.mark_register(io_reg)?;
                builder.push(
                    Instruction::StoreVariable {
                        var_id,
                        src: io_reg,
                    }
                    .into_spanned(catch_span),
                )?;
            }
            // Compile the block, now that the variable is set
            compile_block(working_set, builder, block, redirect_modes, None, io_reg)?;
        }
        Some(CatchType::Closure { closure_reg }) => {
            // We should call `do`. Error will be in io_reg
            let do_decl_id = working_set.find_decl(b"do").ok_or_else(|| {
                CompileError::MissingRequiredDeclaration {
                    decl_name: "do".into(),
                    span: call.head,
                }
            })?;
            builder.mark_register(io_reg)?;

            // Push the closure and the error
            builder
                .push(Instruction::PushPositional { src: closure_reg }.into_spanned(catch_span))?;
            builder.push(Instruction::PushPositional { src: io_reg }.into_spanned(catch_span))?;

            // Empty input to the block
            builder.load_empty(io_reg)?;

            // Call `do $closure $err`
            builder.push(
                Instruction::Call {
                    decl_id: do_decl_id,
                    src_dst: io_reg,
                }
                .into_spanned(catch_span),
            )?;
        }
        None => {
            // Just set out to empty.
            builder.load_empty(io_reg)?;
        }
    }

    // This is the end - if we succeeded, should jump here
    builder.set_branch_target(jump_index, builder.next_instruction_index())?;

    Ok(())
}

/// Compile a call to `loop` (via `jump`)
pub(crate) fn compile_loop(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // LOOP: drain %io_reg
    //       ...<block>...
    //       jump %LOOP
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "loop".into(),
        span: call.head,
    };

    let block_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    let loop_index = builder.drain(io_reg, call.head)?;

    compile_block(
        working_set,
        builder,
        block,
        RedirectModes::default(),
        None,
        io_reg,
    )?;

    builder.jump(loop_index, call.head)?;

    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `while`, via branch instructions
pub(crate) fn compile_while(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // LOOP:  drain %io_reg
    //        %io_reg <- <condition>
    //        branch-if %io_reg, TRUE
    //        jump FALSE
    // TRUE:  ...<block>...
    //        jump LOOP
    // FALSE:
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "while".into(),
        span: call.head,
    };

    let cond_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_arg = call.positional_nth(1).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    let loop_index = builder.drain(io_reg, call.head)?;

    compile_expression(
        working_set,
        builder,
        cond_arg,
        redirect_modes.with_capture_out(call.head),
        None,
        io_reg,
    )?;

    let branch_true_index = builder.branch_if_placeholder(io_reg, call.head)?;
    let jump_false_index = builder.jump_placeholder(call.head)?;

    builder.set_branch_target(branch_true_index, builder.next_instruction_index())?;

    compile_block(
        working_set,
        builder,
        block,
        RedirectModes::default(),
        None,
        io_reg,
    )?;

    builder.jump(loop_index, call.head)?;

    builder.set_branch_target(jump_false_index, builder.next_instruction_index())?;

    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `for` (via `iterate`)
pub(crate) fn compile_for(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    //       %stream_reg <- <in_expr>
    // LOOP: iterate %io_reg, %stream_reg, END
    //       store-variable $var, %io_reg
    //       %io_reg <- <...block...>
    //       drain %io_reg
    //       jump LOOP
    // END:
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "for".into(),
        span: call.head,
    };

    if call.get_named_arg("numbered").is_some() {
        // This is deprecated and we don't support it.
        return Err(invalid());
    }

    let var_decl_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let var_id = var_decl_arg.as_var().ok_or_else(invalid)?;

    let in_arg = call.positional_nth(1).ok_or_else(invalid)?;
    let in_expr = in_arg.as_keyword().ok_or_else(invalid)?;

    let block_arg = call.positional_nth(2).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    // Ensure io_reg is marked so we don't use it
    builder.mark_register(io_reg)?;

    let stream_reg = builder.next_register()?;

    compile_expression(
        working_set,
        builder,
        in_expr,
        redirect_modes.with_capture_out(in_expr.span),
        None,
        stream_reg,
    )?;

    // This gets a value from the stream each time it's executed
    // io_reg basically will act as our scratch register here
    let iterate_index = builder.push(
        Instruction::Iterate {
            dst: io_reg,
            stream: stream_reg,
            end_index: usize::MAX, // placeholder
        }
        .into_spanned(call.head),
    )?;

    // Put the received value in the variable
    builder.push(
        Instruction::StoreVariable {
            var_id,
            src: io_reg,
        }
        .into_spanned(var_decl_arg.span),
    )?;

    // Do the body of the block
    compile_block(
        working_set,
        builder,
        block,
        RedirectModes::default(),
        None,
        io_reg,
    )?;

    // Drain the output
    builder.drain(io_reg, call.head)?;

    // Loop back to iterate to get the next value
    builder.jump(iterate_index, call.head)?;

    // Update the iterate target to the end of the loop
    let target_index = builder.next_instruction_index();
    builder.set_branch_target(iterate_index, target_index)?;

    // We don't need stream_reg anymore, after the loop
    // io_reg is guaranteed to be Empty due to the iterate instruction before
    builder.free_register(stream_reg)?;
    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `return` as a `return` instruction.
///
/// This is not strictly necessary, but it is more efficient.
pub(crate) fn compile_return(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // %io_reg <- <arg_expr>
    // return %io_reg
    if let Some(arg_expr) = call.positional_nth(0) {
        compile_expression(
            working_set,
            builder,
            arg_expr,
            redirect_modes.with_capture_out(arg_expr.span),
            None,
            io_reg,
        )?;
    } else {
        builder.load_empty(io_reg)?;
    }

    builder.push(Instruction::Return { src: io_reg }.into_spanned(call.head))?;

    // io_reg is supposed to remain allocated
    builder.load_empty(io_reg)?;

    Ok(())
}
