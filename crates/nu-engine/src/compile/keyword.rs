use nu_protocol::{
    ast::{Call, Expr, Expression},
    engine::StateWorkingSet,
    ir::Instruction,
    IntoSpanned, RegId,
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
