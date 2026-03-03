use nu_protocol::{
    IntoSpanned, RegId, Span, Type, VarId,
    ast::{Block, Call, Expr, Expression},
    engine::StateWorkingSet,
    ir::Instruction,
};

use super::{BlockBuilder, CompileError, RedirectModes, compile_block, compile_expression};

/// Compile a call to `if` as a branch-if
pub(crate) fn compile_if(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
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

    let true_label = builder.label(None);
    let false_label = builder.label(None);
    let end_label = builder.label(None);

    let not_condition_reg = {
        // Compile the condition first
        let condition_reg = builder.next_register()?;
        compile_expression(
            working_set,
            builder,
            condition,
            RedirectModes::value(condition.span),
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

    // Set up a branch if the condition is false.
    builder.branch_if(not_condition_reg, false_label, call.head)?;
    builder.add_comment("if false");

    // Compile the true case
    builder.set_label(true_label, builder.here())?;
    compile_block(
        working_set,
        builder,
        true_block,
        redirect_modes.clone(),
        Some(io_reg),
        io_reg,
    )?;

    // Add a jump over the false case
    builder.jump(end_label, else_arg.map(|e| e.span).unwrap_or(call.head))?;
    builder.add_comment("end if");

    // On the else side now, assert that io_reg is still valid
    builder.set_label(false_label, builder.here())?;
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

    // Set the end label
    builder.set_label(end_label, builder.here())?;

    Ok(())
}

/// Compile a call to `match`
pub(crate) fn compile_match(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
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
    //         check-match-guard %guard_reg
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
        RedirectModes::value(match_expr.span),
        None,
        match_reg,
    )?;

    // Important to collect it first
    builder.push(Instruction::Collect { src_dst: match_reg }.into_spanned(match_expr.span))?;

    // Generate the `match` instructions. Guards are not used at this stage.
    let mut match_labels = Vec::with_capacity(match_block.len());
    let mut next_labels = Vec::with_capacity(match_block.len());
    let end_label = builder.label(None);

    for (pattern, _) in match_block {
        let match_label = builder.label(None);
        match_labels.push(match_label);
        builder.r#match(
            pattern.pattern.clone(),
            match_reg,
            match_label,
            pattern.span,
        )?;
        // Also add a label for the next match instruction or failure case
        next_labels.push(builder.label(Some(builder.here())));
    }

    // Match fall-through to jump to the end, if no match
    builder.load_empty(io_reg)?;
    builder.drop_reg(match_reg)?;
    builder.jump(end_label, call.head)?;

    // Generate each of the match expressions. Handle guards here, if present.
    for (index, (pattern, expr)) in match_block.iter().enumerate() {
        let match_label = match_labels[index];
        let next_label = next_labels[index];

        // `io_reg` and `match_reg` are still valid at each of these branch targets
        builder.mark_register(io_reg)?;
        builder.mark_register(match_reg)?;

        // Set the original match instruction target here
        builder.set_label(match_label, builder.here())?;

        // Handle guard, if present
        if let Some(guard) = &pattern.guard {
            let guard_reg = builder.next_register()?;
            compile_expression(
                working_set,
                builder,
                guard,
                RedirectModes::value(guard.span),
                None,
                guard_reg,
            )?;
            builder
                .push(Instruction::CheckMatchGuard { src: guard_reg }.into_spanned(guard.span))?;
            builder.push(Instruction::Not { src_dst: guard_reg }.into_spanned(guard.span))?;
            // Branch to the next match instruction if the branch fails to match
            builder.branch_if(
                guard_reg,
                next_label,
                // Span the branch with the next pattern, or the head if this is the end
                match_block
                    .get(index + 1)
                    .map(|b| b.0.span)
                    .unwrap_or(call.head),
            )?;
            builder.add_comment("if match guard false");
        }

        // match_reg no longer needed, successful match
        builder.drop_reg(match_reg)?;

        // Execute match right hand side expression
        if let Expr::Block(block_id) = expr.expr {
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

        // Jump to the end after the match logic is done
        builder.jump(end_label, call.head)?;
        builder.add_comment("end match");
    }

    // Set the end destination
    builder.set_label(end_label, builder.here())?;

    Ok(())
}

/// Compile a call to `let` or `mut`.
///
/// This supports two syntax forms:
/// 1. `let var = expr` - Evaluates expr and stores result in variable (no output)
/// 2. `input | let var` - Uses pipeline input as the value (passes value through)
///
/// Output behavior:
/// - `let x = expr`: stores the value and produces **no output** (Empty)
/// - `input | let x`: stores the value and **passes it through** to the next pipeline element
///
/// # Arguments
/// * `working_set` - The current state working set
/// * `builder` - The IR block builder
/// * `call` - The parsed call to `let` or `mut`
/// * `_redirect_modes` - Output redirection modes (unused)
/// * `input_reg` - Optional register containing pipeline input (for `input | let var` form)
/// * `io_reg` - The I/O register for the operation result
pub(crate) fn compile_let(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // Case 1: `let var = expr` (no output)
    //   %io_reg <- ...<block>...
    //   store-variable $var, %io_reg
    //   load-empty %io_reg
    //
    // Case 2: `input | let var` (pass through)
    //   collect %input_reg
    //   move %io_reg, %input_reg (if different)
    //   store-variable $var, %io_reg
    //   load-variable %io_reg, $var
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "let".into(),
        span: call.head,
    };

    let var_decl_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let var_id = var_decl_arg.as_var().ok_or_else(invalid)?;

    // Handle the two syntax forms:
    // 1. `let var = expr`: compile expr and store result
    // 2. `let var` (no =): use input_reg as the value
    let has_initial_value = call.positional_nth(1).is_some();
    if has_initial_value {
        // Safe to use expect here since we just checked is_some()
        let block_arg = call.positional_nth(1).expect("checked above");
        let block_id = block_arg.as_block().ok_or_else(invalid)?;
        let block = working_set.get_block(block_id);

        // Pass the input_reg to the block so expressions like `let x = (str length)`
        // can access the pipeline input from the enclosing context
        compile_block(
            working_set,
            builder,
            block,
            RedirectModes::value(call.head),
            input_reg,
            io_reg,
        )?;
    } else if let Some(input_reg) = input_reg {
        // For `let var` without =, assign the input value
        builder.push(Instruction::Collect { src_dst: input_reg }.into_spanned(call.head))?;
        if input_reg != io_reg {
            builder.push(
                Instruction::Move {
                    dst: io_reg,
                    src: input_reg,
                }
                .into_spanned(call.head),
            )?;
        }
    }
    // If no initial_value and no input_reg, io_reg should already be empty (this shouldn't normally occur)

    let variable = working_set.get_variable(var_id);

    // If the variable is annotated with type `glob`, convert the value to
    // a `Glob` (expandable) *before* storing.  We use `GlobFrom { no_expand: false }`
    // so the stored Value::Glob behaves like a glob literal and will expand at
    // runtime (e.g. `let g: glob = "*.toml"; ls $g` should expand).  This
    // mirrors the `into glob` conversion and matches interpreter coercion in
    // `nu-cmd-lang::let` (see tests in `nu-command`).
    if variable.ty == Type::Glob {
        builder.push(
            Instruction::GlobFrom {
                src_dst: io_reg,
                no_expand: false,
            }
            .into_spanned(call.head),
        )?;
    }

    builder.push(
        Instruction::StoreVariable {
            var_id,
            src: io_reg,
        }
        .into_spanned(call.head),
    )?;
    builder.add_comment("let");

    if has_initial_value {
        // `let var = expr`: suppress output (traditional assignment, no display)
        builder.load_empty(io_reg)?;
    } else {
        // `input | let var`: pass through the assigned value to the next pipeline element
        builder.push(
            Instruction::LoadVariable {
                dst: io_reg,
                var_id,
            }
            .into_spanned(call.head),
        )?;
    }

    Ok(())
}

/// Compile a call to `try`, setting an error handler over the evaluated block
pub(crate) fn compile_try(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode (literal block):
    //
    //       on-error-into ERR, %io_reg           // or without
    //       finally-into  FINALLY, $io_reg       // or without
    //       %io_reg <- <...block...> <- %io_reg
    //       write-to-out-dests %io_reg
    //       pop-error-handler
    //       jump END
    // ERR:  clone %err_reg, %io_reg
    //       store-variable $err_var, %err_reg         // or without
    //       %io_reg <- <...catch block...> <- %io_reg // set to empty if no catch block
    //       pop-finally
    // END:
    //
    // with expression that can't be inlined:
    //
    //       %closure_reg <- <catch_expr>
    //       on-error-into ERR, %io_reg
    //       finally-into  FINALLY, $io_reg
    //       %io_reg <- <...block...> <- %io_reg
    //       write-to-out-dests %io_reg
    //       pop-error-handler
    //       jump END
    // ERR:  clone %err_reg, %io_reg
    //       push-positional %closure_reg
    //       push-positional %err_reg
    //       call "do", %io_reg
    //       pop-finally
    // END:
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "try".into(),
        span: call.head,
    };

    let block_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    // manually parsing for `catch` or `finally`.
    let mut catch_expr = None;
    let mut finally_expr = None;
    if let Some(kw_expr) = call.positional_nth(1) {
        let (keyword, expr) = kw_expr.as_keyword_with_name().ok_or_else(invalid)?;
        if keyword == b"catch" {
            catch_expr = Some(expr);
        } else if keyword == b"finally" {
            finally_expr = Some(expr);
        }
    };
    if let Some(kw_expr) = call.positional_nth(2) {
        let (keyword, expr) = kw_expr.as_keyword_with_name().ok_or_else(invalid)?;
        if keyword == b"catch" {
            // just deny it, because it should only be valid in 1st positional arguments.
            return Err(invalid());
        } else if keyword == b"finally" {
            // deny duplicate finally.
            if finally_expr.is_some() {
                return Err(invalid());
            }
            finally_expr = Some(expr);
        }
    };

    let catch_span = catch_expr.map(|e| e.span).unwrap_or(call.head);

    let err_label = builder.label(None);
    let end_label = builder.label(None);

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
                    RedirectModes::value(catch_expr.span),
                    None,
                    closure_reg,
                )?;
                Ok(CatchType::Closure { closure_reg })
            }
        })
        .transpose()?;

    struct FinallyInfo<'a> {
        block: &'a Block,
        var_id: Option<VarId>,
    }
    let finally_type = finally_expr
        .map(|finally_expr| match finally_expr.as_block() {
            Some(block_id) => {
                let block = working_set.get_block(block_id);
                let var_id = block.signature.get_positional(0).and_then(|v| v.var_id);
                Ok(FinallyInfo { block, var_id })
            }
            None => Err(invalid()),
        })
        .transpose()?;

    // Put the error handler instruction. If we have a catch expression then we should capture the
    // error.
    let mut has_try_comment = false;
    if catch_type.is_some() {
        builder.push(
            Instruction::OnErrorInto {
                index: err_label.0,
                dst: io_reg,
            }
            .into_spanned(call.head),
        )?;
        builder.add_comment("try");
        has_try_comment = true;
    } else if finally_expr.is_none() {
        // Simply try, without `catch` and `finally` block, need to set up OnErrorHandler.
        // so `try { 1 / 0 }` works
        builder.push(Instruction::OnError { index: err_label.0 }.into_spanned(call.head))?;
        builder.add_comment("try");
        has_try_comment = true;
    };

    builder.begin_try();

    if let Some(finally_info) = &finally_type {
        if finally_info.var_id.is_some() {
            builder.push(
                Instruction::FinallyInto {
                    index: end_label.0,
                    dst: io_reg,
                }
                .into_spanned(call.head),
            )?;
        } else {
            builder.push(Instruction::Finally { index: end_label.0 }.into_spanned(call.head))?;
        }
        if !has_try_comment {
            builder.add_comment("try");
        }
    }

    // Compile the block
    compile_block(
        working_set,
        builder,
        block,
        redirect_modes.clone(),
        Some(io_reg),
        io_reg,
    )?;

    // Successful case:
    // - write to the current output destinations
    // - pop the error handler
    if let Some(mode) = redirect_modes.out {
        builder.push(mode.map(|mode| Instruction::RedirectOut { mode }))?;
    }

    if let Some(mode) = redirect_modes.err {
        builder.push(mode.map(|mode| Instruction::RedirectErr { mode }))?;
    }

    if finally_type.is_none() {
        builder.push(Instruction::DrainIfEnd { src: io_reg }.into_spanned(call.head))?;
    }
    if catch_expr.is_some() {
        builder.push(Instruction::PopErrorHandler.into_spanned(call.head))?;
    }

    builder.end_try()?;

    // Jump over the failure case
    builder.jump(end_label, catch_span)?;

    // This is the error handler
    builder.set_label(err_label, builder.here())?;

    // Mark out register as likely not clean - state in error handler is not well defined
    builder.mark_register(io_reg)?;

    // Now compile whatever is necessary for the error handler
    match catch_type {
        Some(CatchType::Block { block, var_id }) => {
            // Error will be in io_reg
            builder.mark_register(io_reg)?;
            if let Some(var_id) = var_id {
                // Take a copy of the error as $err, since it will also be input
                let err_reg = builder.next_register()?;
                builder.push(
                    Instruction::Clone {
                        dst: err_reg,
                        src: io_reg,
                    }
                    .into_spanned(catch_span),
                )?;
                builder.push(
                    Instruction::StoreVariable {
                        var_id,
                        src: err_reg,
                    }
                    .into_spanned(catch_span),
                )?;
            }
            // Compile the block, now that the variable is set
            compile_block(
                working_set,
                builder,
                block,
                redirect_modes.clone(),
                Some(io_reg),
                io_reg,
            )?;
        }
        Some(CatchType::Closure { closure_reg }) => {
            compile_closure_call(working_set, builder, call, io_reg, closure_reg, catch_span)?
        }
        None => {
            // Just set out to empty.
            builder.load_empty(io_reg)?;
        }
    }
    if finally_type.is_some() {
        builder.push(Instruction::PopFinallyRun.into_spanned(call.head))?;
    }

    // This is the end - whatever we succeeded or not, should jump here for finally clause.
    builder.set_label(end_label, builder.here())?;
    // This is the finally part.
    if let Some(finally_part) = finally_type {
        if let Some(var_id) = finally_part.var_id {
            let value_reg = builder.next_register()?;
            builder.push(
                Instruction::Clone {
                    dst: value_reg,
                    src: io_reg,
                }
                .into_spanned(call.head),
            )?;
            builder.push(
                Instruction::StoreVariable {
                    var_id,
                    src: value_reg,
                }
                .into_spanned(call.head),
            )?;
        }
        compile_block(
            working_set,
            builder,
            finally_part.block,
            redirect_modes,
            Some(io_reg),
            io_reg,
        )?;
    }

    Ok(())
}

fn compile_closure_call(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    io_reg: RegId,
    closure_reg: RegId,
    span: Span,
) -> Result<(), CompileError> {
    // We should call `do`. Error will be in io_reg
    let do_decl_id =
        working_set
            .find_decl(b"do")
            .ok_or_else(|| CompileError::MissingRequiredDeclaration {
                decl_name: "do".into(),
                span: call.head,
            })?;

    // Take a copy of io_reg, because we pass it both as an argument and input
    builder.mark_register(io_reg)?;
    let arg_reg = builder.next_register()?;
    builder.push(
        Instruction::Clone {
            dst: arg_reg,
            src: io_reg,
        }
        .into_spanned(span),
    )?;

    // Push the closure and the argument
    builder.push(Instruction::PushPositional { src: closure_reg }.into_spanned(span))?;
    builder.push(Instruction::PushPositional { src: arg_reg }.into_spanned(span))?;

    // Call `$err | do $closure $arg`
    builder.push(
        Instruction::Call {
            decl_id: do_decl_id,
            src_dst: io_reg,
        }
        .into_spanned(span),
    )?;

    Ok(())
}

/// Compile a call to `loop` (via `jump`)
pub(crate) fn compile_loop(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    //       drop %io_reg
    // LOOP: %io_reg <- ...<block>...
    //       drain %io_reg
    //       jump %LOOP
    // END:  drop %io_reg
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "loop".into(),
        span: call.head,
    };

    let block_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    let loop_ = builder.begin_loop();
    builder.load_empty(io_reg)?;

    builder.set_label(loop_.continue_label, builder.here())?;

    compile_block(
        working_set,
        builder,
        block,
        RedirectModes::default(),
        None,
        io_reg,
    )?;

    // Drain the output, just like for a semicolon
    builder.drain(io_reg, call.head)?;

    builder.jump(loop_.continue_label, call.head)?;
    builder.add_comment("loop");

    builder.set_label(loop_.break_label, builder.here())?;
    builder.end_loop(loop_)?;

    // State of %io_reg is not necessarily well defined here due to control flow, so make sure it's
    // empty.
    builder.mark_register(io_reg)?;
    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `while`, via branch instructions
pub(crate) fn compile_while(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // LOOP:  %io_reg <- <condition>
    //        branch-if %io_reg, TRUE
    //        jump FALSE
    // TRUE:  %io_reg <- ...<block>...
    //        drain %io_reg
    //        jump LOOP
    // FALSE: drop %io_reg
    let invalid = || CompileError::InvalidKeywordCall {
        keyword: "while".into(),
        span: call.head,
    };

    let cond_arg = call.positional_nth(0).ok_or_else(invalid)?;
    let block_arg = call.positional_nth(1).ok_or_else(invalid)?;
    let block_id = block_arg.as_block().ok_or_else(invalid)?;
    let block = working_set.get_block(block_id);

    let loop_ = builder.begin_loop();
    builder.set_label(loop_.continue_label, builder.here())?;

    let true_label = builder.label(None);

    compile_expression(
        working_set,
        builder,
        cond_arg,
        RedirectModes::value(call.head),
        None,
        io_reg,
    )?;

    builder.branch_if(io_reg, true_label, call.head)?;
    builder.add_comment("while");
    builder.jump(loop_.break_label, call.head)?;
    builder.add_comment("end while");

    builder.load_empty(io_reg)?;

    builder.set_label(true_label, builder.here())?;

    compile_block(
        working_set,
        builder,
        block,
        RedirectModes::default(),
        None,
        io_reg,
    )?;

    // Drain the result, just like for a semicolon
    builder.drain(io_reg, call.head)?;

    builder.jump(loop_.continue_label, call.head)?;
    builder.add_comment("while");

    builder.set_label(loop_.break_label, builder.here())?;
    builder.end_loop(loop_)?;

    // State of %io_reg is not necessarily well defined here due to control flow, so make sure it's
    // empty.
    builder.mark_register(io_reg)?;
    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `for` (via `iterate`)
pub(crate) fn compile_for(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
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
    // END:  drop %io_reg
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
    builder.load_empty(io_reg)?;

    let stream_reg = builder.next_register()?;

    compile_expression(
        working_set,
        builder,
        in_expr,
        RedirectModes::caller(in_expr.span),
        None,
        stream_reg,
    )?;

    // Set up loop state
    let loop_ = builder.begin_loop();
    builder.set_label(loop_.continue_label, builder.here())?;

    // This gets a value from the stream each time it's executed
    // io_reg basically will act as our scratch register here
    builder.push(
        Instruction::Iterate {
            dst: io_reg,
            stream: stream_reg,
            end_index: loop_.break_label.0,
        }
        .into_spanned(call.head),
    )?;
    builder.add_comment("for");

    // Put the received value in the variable
    builder.push(
        Instruction::StoreVariable {
            var_id,
            src: io_reg,
        }
        .into_spanned(var_decl_arg.span),
    )?;

    builder.load_empty(io_reg)?;

    // Do the body of the block
    compile_block(
        working_set,
        builder,
        block,
        RedirectModes::default(),
        None,
        io_reg,
    )?;

    // Drain the output, just like for a semicolon
    builder.drain(io_reg, call.head)?;

    // Loop back to iterate to get the next value
    builder.jump(loop_.continue_label, call.head)?;

    // Set the end of the loop
    builder.set_label(loop_.break_label, builder.here())?;
    builder.end_loop(loop_)?;

    // We don't need stream_reg anymore, after the loop
    // io_reg may or may not be empty, so be sure it is
    builder.free_register(stream_reg)?;
    builder.mark_register(io_reg)?;
    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `break`.
pub(crate) fn compile_break(
    _working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    if !builder.is_in_loop() {
        return Err(CompileError::NotInALoop {
            msg: "'break' can only be used inside a loop".to_string(),
            span: Some(call.head),
        });
    }
    builder.load_empty(io_reg)?;
    for _ in 0..builder.context_stack.try_block_depth_from_loop() {
        builder.push(Instruction::PopErrorHandler.into_spanned(call.head))?;
    }
    builder.push_break(call.head)?;
    builder.add_comment("break");
    Ok(())
}

/// Compile a call to `continue`.
pub(crate) fn compile_continue(
    _working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    if !builder.is_in_loop() {
        return Err(CompileError::NotInALoop {
            msg: "'continue' can only be used inside a loop".to_string(),
            span: Some(call.head),
        });
    }
    builder.load_empty(io_reg)?;
    for _ in 0..builder.context_stack.try_block_depth_from_loop() {
        builder.push(Instruction::PopErrorHandler.into_spanned(call.head))?;
    }
    builder.push_continue(call.head)?;
    builder.add_comment("continue");
    Ok(())
}

/// Compile a call to `return` as a `return-early` instruction.
///
/// This is not strictly necessary, but it is more efficient.
pub(crate) fn compile_return(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    _input_reg: Option<RegId>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode:
    //
    // %io_reg <- <arg_expr>
    // return-early %io_reg
    if let Some(arg_expr) = call.positional_nth(0) {
        compile_expression(
            working_set,
            builder,
            arg_expr,
            RedirectModes::value(arg_expr.span),
            None,
            io_reg,
        )?;
    } else {
        builder.load_empty(io_reg)?;
    }

    // TODO: It would be nice if this could be `return` instead, but there is a little bit of
    // behaviour remaining that still depends on `ShellError::Return`
    builder.push(Instruction::ReturnEarly { src: io_reg }.into_spanned(call.head))?;

    // io_reg is supposed to remain allocated
    builder.load_empty(io_reg)?;

    Ok(())
}

/// Compile a call to `collect` as a `Collect` instruction.
///
/// This makes it possible to check pipefail exit status when calling the
/// `collect` command.
pub(crate) fn compile_collect(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    _redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Pseudocode (no closure argument):
    //
    // collect %io_reg
    //
    // With closure argument:
    //
    // collect %io_reg
    // %closure_reg <- <arg expr>
    // clone %arg_reg, %io_reg
    // push-positional %closure_reg
    // push-positional %arg_reg
    // call "do", %io_reg

    builder.push(Instruction::Collect { src_dst: io_reg }.into_spanned(call.head))?;

    if let Some(arg_expr) = call.positional_nth(0) {
        // We have to compile the expression into a closure,
        // then compile a closure call
        let closure_reg = builder.next_register()?;
        compile_expression(
            working_set,
            builder,
            arg_expr,
            RedirectModes::value(arg_expr.span),
            None,
            closure_reg,
        )?;
        compile_closure_call(
            working_set,
            builder,
            call,
            io_reg,
            closure_reg,
            arg_expr.span,
        )?;
    }

    Ok(())
}
