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
    let index_of_jump = builder.push(
        Instruction::Jump { index: usize::MAX }
            .into_spanned(else_arg.map(|e| e.span).unwrap_or(call.head)),
    )?;

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

    Ok(())
}
