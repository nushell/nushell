use std::{iter::repeat, sync::Arc};

use nu_protocol::{
    ast::{Argument, Block, Call, Expression, ExternalArgument},
    engine::StateWorkingSet,
    ir::{Instruction, IrAstRef, Literal},
    IntoSpanned, RegId, Span, Spanned,
};

use super::{compile_expression, keyword::*, BlockBuilder, CompileError, RedirectModes};

pub(crate) fn compile_call(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    let decl = working_set.get_decl(call.decl_id);

    // Check if this call has --help - if so, just redirect to `help`
    if call.named_iter().any(|(name, _, _)| name.item == "help") {
        return compile_help(
            working_set,
            builder,
            decl.name().into_spanned(call.head),
            io_reg,
        );
    }

    // Try to figure out if this is a keyword call like `if`, and handle those specially
    if decl.is_keyword() {
        match decl.name() {
            "if" => {
                return compile_if(working_set, builder, call, redirect_modes, io_reg);
            }
            "match" => {
                return compile_match(working_set, builder, call, redirect_modes, io_reg);
            }
            "const" => {
                // This differs from the behavior of the const command, which adds the const value
                // to the stack. Since `load-variable` also checks `engine_state` for the variable
                // and will get a const value though, is it really necessary to do that?
                return builder.load_empty(io_reg);
            }
            "alias" => {
                // Alias does nothing
                return builder.load_empty(io_reg);
            }
            "let" | "mut" => {
                return compile_let(working_set, builder, call, redirect_modes, io_reg);
            }
            "try" => {
                return compile_try(working_set, builder, call, redirect_modes, io_reg);
            }
            "loop" => {
                return compile_loop(working_set, builder, call, redirect_modes, io_reg);
            }
            "while" => {
                return compile_while(working_set, builder, call, redirect_modes, io_reg);
            }
            "for" => {
                return compile_for(working_set, builder, call, redirect_modes, io_reg);
            }
            "return" => {
                return compile_return(working_set, builder, call, redirect_modes, io_reg);
            }
            _ => (),
        }
    }

    // Also, if this is a custom command (block) call, we should handle this completely differently,
    // setting args in variables on a callee stack.
    if let Some(block_id) = decl.block_id() {
        let block = working_set.get_block(block_id);
        return compile_custom_command_call(
            working_set,
            builder,
            block,
            call,
            redirect_modes,
            io_reg,
        );
    }

    // Keep AST if the decl needs it.
    let requires_ast = decl.requires_ast_for_arguments();

    // It's important that we evaluate the args first before trying to set up the argument
    // state for the call.
    //
    // We could technically compile anything that isn't another call safely without worrying about
    // the argument state, but we'd have to check all of that first and it just isn't really worth
    // it.
    enum CompiledArg<'a> {
        Positional(RegId, Span, Option<IrAstRef>),
        Named(&'a str, Option<RegId>, Span, Option<IrAstRef>),
        Spread(RegId, Span, Option<IrAstRef>),
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

        let ast_ref = arg
            .expr()
            .filter(|_| requires_ast)
            .map(|expr| IrAstRef(Arc::new(expr.clone())));

        match arg {
            Argument::Positional(_) | Argument::Unknown(_) => {
                compiled_args.push(CompiledArg::Positional(
                    arg_reg.expect("expr() None in non-Named"),
                    arg.span(),
                    ast_ref,
                ))
            }
            Argument::Named((name, _, _)) => compiled_args.push(CompiledArg::Named(
                name.item.as_str(),
                arg_reg,
                arg.span(),
                ast_ref,
            )),
            Argument::Spread(_) => compiled_args.push(CompiledArg::Spread(
                arg_reg.expect("expr() None in non-Named"),
                arg.span(),
                ast_ref,
            )),
        }
    }

    // Now that the args are all compiled, set up the call state (argument stack and redirections)
    for arg in compiled_args {
        match arg {
            CompiledArg::Positional(reg, span, ast_ref) => {
                builder.push(Instruction::PushPositional { src: reg }.into_spanned(span))?;
                builder.set_last_ast(ast_ref);
            }
            CompiledArg::Named(name, Some(reg), span, ast_ref) => {
                let name = builder.data(name)?;
                builder.push(Instruction::PushNamed { name, src: reg }.into_spanned(span))?;
                builder.set_last_ast(ast_ref);
            }
            CompiledArg::Named(name, None, span, ast_ref) => {
                let name = builder.data(name)?;
                builder.push(Instruction::PushFlag { name }.into_spanned(span))?;
                builder.set_last_ast(ast_ref);
            }
            CompiledArg::Spread(reg, span, ast_ref) => {
                builder.push(Instruction::AppendRest { src: reg }.into_spanned(span))?;
                builder.set_last_ast(ast_ref);
            }
        }
    }

    // Add any parser info from the call
    for (name, info) in &call.parser_info {
        let name = builder.data(name)?;
        let info = Box::new(info.clone());
        builder.push(Instruction::PushParserInfo { name, info }.into_spanned(call.head))?;
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

pub(crate) fn compile_help(
    working_set: &StateWorkingSet<'_>,
    builder: &mut BlockBuilder,
    decl_name: Spanned<&str>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    let help_command_id =
        working_set
            .find_decl(b"help")
            .ok_or_else(|| CompileError::MissingRequiredDeclaration {
                decl_name: "help".into(),
                span: decl_name.span,
            })?;

    let name_data = builder.data(decl_name.item)?;
    let name_literal = builder.literal(decl_name.map(|_| Literal::String(name_data)))?;

    builder.push(Instruction::PushPositional { src: name_literal }.into_spanned(decl_name.span))?;

    builder.push(
        Instruction::Call {
            decl_id: help_command_id,
            src_dst: io_reg,
        }
        .into_spanned(decl_name.span),
    )?;

    Ok(())
}

pub(crate) fn compile_custom_command_call(
    working_set: &StateWorkingSet<'_>,
    builder: &mut BlockBuilder,
    decl_block: &Block,
    call: &Call,
    redirect_modes: RedirectModes,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // Custom command calls take place on a new stack
    builder.push(Instruction::NewCalleeStack.into_spanned(call.head))?;

    // Add captures
    for &var_id in &decl_block.captures {
        builder.push(Instruction::CaptureVariable { var_id }.into_spanned(call.head))?;
    }

    // Add positional arguments
    let positional_iter = decl_block
        .signature
        .required_positional
        .iter()
        .chain(&decl_block.signature.optional_positional)
        .zip(call.positional_iter().map(Some).chain(repeat(None)));

    for (declared_positional, provided_positional) in positional_iter {
        let var_id = declared_positional
            .var_id
            .expect("custom command positional parameter missing var_id");

        let var_reg = builder.next_register()?;
        let span = provided_positional.map(|b| b.span).unwrap_or(call.head);
        if let Some(expr) = provided_positional {
            // If the arg was provided, compile it
            compile_expression(
                working_set,
                builder,
                expr,
                redirect_modes.with_capture_out(span),
                None,
                var_reg,
            )?;
        } else {
            if let Some(default) = declared_positional.default_value.clone() {
                // Put the default as a Value if it's present
                builder.push(
                    Instruction::LoadValue {
                        dst: var_reg,
                        val: Box::new(default),
                    }
                    .into_spanned(span),
                )?;
            } else {
                // It doesn't really matter if this is a required argument, because we're supposed
                // to check that in the parser
                builder.load_literal(var_reg, Literal::Nothing.into_spanned(span))?;
            }
        }
        builder.push(
            Instruction::PushVariable {
                var_id,
                src: var_reg,
            }
            .into_spanned(span),
        )?;
    }

    // Add rest arguments
    if let Some(declared_rest) = &decl_block.signature.rest_positional {
        let var_id = declared_rest
            .var_id
            .expect("custom command rest parameter missing var_id");

        let rest_index = decl_block.signature.required_positional.len()
            + decl_block.signature.optional_positional.len();

        // Use the first span of rest if possible
        let span = call
            .rest_iter(rest_index)
            .next()
            .map(|(e, _)| e.span)
            .unwrap_or(call.head);

        // Build the rest list from the remaining arguments
        let var_reg = builder.literal(Literal::List { capacity: 0 }.into_spanned(span))?;

        for (expr, spread) in call.rest_iter(rest_index) {
            let expr_reg = builder.next_register()?;
            compile_expression(
                working_set,
                builder,
                expr,
                redirect_modes.with_capture_out(expr.span),
                None,
                expr_reg,
            )?;
            builder.push(
                // If the original argument was a spread argument, spread it into the list
                if spread {
                    Instruction::ListSpread {
                        src_dst: var_reg,
                        items: var_reg,
                    }
                } else {
                    Instruction::ListPush {
                        src_dst: var_reg,
                        item: expr_reg,
                    }
                }
                .into_spanned(expr.span),
            )?;
        }

        builder.push(
            Instruction::PushVariable {
                var_id,
                src: var_reg,
            }
            .into_spanned(span),
        )?;
    }

    // Add named arguments
    for flag in &decl_block.signature.named {
        if let Some(var_id) = flag.var_id {
            let var_reg = builder.next_register()?;

            let provided_arg = call
                .named_iter()
                .find(|(name, _, _)| name.item == flag.long);

            let span = provided_arg
                .map(|(name, _, _)| name.span)
                .unwrap_or(call.head);

            if let Some(expr) = provided_arg.and_then(|(_, _, expr)| expr.as_ref()) {
                // It was provided - compile it
                compile_expression(
                    working_set,
                    builder,
                    expr,
                    redirect_modes.with_capture_out(expr.span),
                    None,
                    var_reg,
                )?;
            } else {
                // It wasn't provided - use the default or `true`
                if let Some(default) = flag.default_value.clone() {
                    builder.push(
                        Instruction::LoadValue {
                            dst: var_reg,
                            val: Box::new(default),
                        }
                        .into_spanned(span),
                    )?;
                } else {
                    builder.load_literal(var_reg, Literal::Bool(true).into_spanned(span))?;
                }
            }

            builder.push(
                Instruction::PushVariable {
                    var_id,
                    src: var_reg,
                }
                .into_spanned(span),
            )?;
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

pub(crate) fn compile_external_call(
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
        .ok_or_else(|| CompileError::RunExternalNotFound { span: head.span })?;

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
