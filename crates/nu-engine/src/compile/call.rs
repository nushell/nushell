use std::sync::Arc;

use nu_protocol::{
    IntoSpanned, RegId, Span, Spanned,
    ast::{Argument, Call, Expression, ExternalArgument},
    engine::StateWorkingSet,
    ir::{Instruction, IrAstRef, Literal},
};

use super::{BlockBuilder, CompileError, RedirectModes, compile_expression, keyword::*};

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
        let name = working_set
            .find_decl_name(call.decl_id) // check for name in scope
            .and_then(|name| std::str::from_utf8(name).ok())
            .unwrap_or(decl.name()); // fall back to decl's name
        return compile_help(working_set, builder, name.into_spanned(call.head), io_reg);
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
            "const" | "export const" => {
                // This differs from the behavior of the const command, which adds the const value
                // to the stack. Since `load-variable` also checks `engine_state` for the variable
                // and will get a const value though, is it really necessary to do that?
                return builder.load_empty(io_reg);
            }
            "alias" | "export alias" => {
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
            "break" => {
                return compile_break(working_set, builder, call, redirect_modes, io_reg);
            }
            "continue" => {
                return compile_continue(working_set, builder, call, redirect_modes, io_reg);
            }
            "return" => {
                return compile_return(working_set, builder, call, redirect_modes, io_reg);
            }
            "def" | "export def" => {
                return builder.load_empty(io_reg);
            }
            _ => (),
        }
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
        Named(
            &'a str,
            Option<&'a str>,
            Option<RegId>,
            Span,
            Option<IrAstRef>,
        ),
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
                    RedirectModes::value(arg.span()),
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
            Argument::Named((name, short, _)) => compiled_args.push(CompiledArg::Named(
                &name.item,
                short.as_ref().map(|spanned| spanned.item.as_str()),
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
            CompiledArg::Named(name, short, Some(reg), span, ast_ref) => {
                if !name.is_empty() {
                    let name = builder.data(name)?;
                    builder.push(Instruction::PushNamed { name, src: reg }.into_spanned(span))?;
                } else {
                    let short = builder.data(short.unwrap_or(""))?;
                    builder
                        .push(Instruction::PushShortNamed { short, src: reg }.into_spanned(span))?;
                }
                builder.set_last_ast(ast_ref);
            }
            CompiledArg::Named(name, short, None, span, ast_ref) => {
                if !name.is_empty() {
                    let name = builder.data(name)?;
                    builder.push(Instruction::PushFlag { name }.into_spanned(span))?;
                } else {
                    let short = builder.data(short.unwrap_or(""))?;
                    builder.push(Instruction::PushShortFlag { short }.into_spanned(span))?;
                }
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
        .ok_or(CompileError::RunExternalNotFound { span: head.span })?;

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
