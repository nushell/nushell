use nu_protocol::{
    ast::{Argument, Call, Expression, ExternalArgument},
    engine::StateWorkingSet,
    ir::Instruction,
    IntoSpanned, RegId, Span,
};

use super::{compile_expression, keyword::*, BlockBuilder, CompileError, RedirectModes};

pub(crate) fn compile_call(
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
            "const" => {
                // This differs from the behavior of the const command, which adds the const value
                // to the stack. Since `load-variable` also checks `engine_state` for the variable
                // and will get a const value though, is it really necessary to do that?
                builder.load_empty(io_reg)?;
                return Ok(());
            }
            "let" | "mut" => {
                return compile_let(working_set, builder, call, redirect_modes, io_reg);
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
            Argument::Unknown(_) => return Err(CompileError::Garbage { span: arg.span() }),
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
