use nu_protocol::{
    ast::{
        Argument, Block, Call, Expr, Expression, Operator, Pipeline, PipelineRedirection,
        RedirectionSource, RedirectionTarget,
    },
    engine::EngineState,
    ir::{Instruction, IrBlock, Literal, RedirectMode},
    IntoSpanned, OutDest, RegId, ShellError, Span, Spanned,
};

const BLOCK_INPUT: RegId = RegId(0);

/// Compile Nushell pipeline abstract syntax tree (AST) to internal representation (IR) instructions
/// for evaluation.
pub fn compile(engine_state: &EngineState, block: &Block) -> Result<IrBlock, ShellError> {
    let mut builder = BlockBuilder::new();

    compile_block(engine_state, &mut builder, block, BLOCK_INPUT)
        .map_err(|err| err.to_shell_error(block.span))?;

    Ok(builder.finish())
}

fn compile_block(
    engine_state: &EngineState,
    builder: &mut BlockBuilder,
    block: &Block,
    input: RegId,
) -> Result<(), CompileError> {
    let span = block.span.unwrap_or(Span::unknown());
    let io_reg = input;
    if !block.pipelines.is_empty() {
        let last_index = block.pipelines.len() - 1;
        for (index, pipeline) in block.pipelines.iter().enumerate() {
            compile_pipeline(engine_state, builder, pipeline, span, io_reg)?;

            if index != last_index {
                // Explicitly drain the I/O reg after each non-final pipeline, and replace
                // with Nothing, because that's how the semicolon functions.
                builder.push(Instruction::Drain { src: io_reg }.into_spanned(span))?;
                builder.push(
                    Instruction::LoadLiteral {
                        dst: io_reg,
                        lit: Literal::Nothing,
                    }
                    .into_spanned(span),
                )?;
            }
        }
    }
    builder.push(Instruction::Return { src: io_reg }.into_spanned(span))
}

fn compile_pipeline(
    engine_state: &EngineState,
    builder: &mut BlockBuilder,
    pipeline: &Pipeline,
    fallback_span: Span,
    io_reg: RegId,
) -> Result<(), CompileError> {
    let mut iter = pipeline.elements.iter().peekable();
    while let Some(element) = iter.next() {
        let span = element.pipe.unwrap_or(fallback_span);

        // We have to get the redirection mode from either the explicit redirection in the pipeline
        // element, or from the next expression if it's specified there.

        let (out_mode_next, err_mode_next) = if let Some(next_element) = iter.peek() {
            redirect_mode_of_expression(engine_state, &next_element.expr)?
        } else {
            (None, None)
        };

        let (out_mode_spec, err_mode_spec) = match &element.redirection {
            Some(PipelineRedirection::Single { source, target }) => {
                let mode = redirection_target_to_mode(engine_state, builder, target, false)?;
                match source {
                    RedirectionSource::Stdout => (Some(mode), None),
                    RedirectionSource::Stderr => (None, Some(mode)),
                    RedirectionSource::StdoutAndStderr => (Some(mode), Some(mode)),
                }
            }
            Some(PipelineRedirection::Separate { out, err }) => {
                let out = redirection_target_to_mode(engine_state, builder, out, true)?;
                let err = redirection_target_to_mode(engine_state, builder, err, true)?;
                (Some(out), Some(err))
            }
            None => (None, None),
        };

        let out_mode = out_mode_spec.or(out_mode_next.map(|mode| mode.into_spanned(span)));
        let err_mode = err_mode_spec.or(err_mode_next.map(|mode| mode.into_spanned(span)));

        compile_expression(
            engine_state,
            builder,
            &element.expr,
            out_mode,
            err_mode,
            Some(io_reg),
            io_reg,
        )?;
    }
    Ok(())
}

fn redirection_target_to_mode(
    engine_state: &EngineState,
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
                engine_state,
                builder,
                expr,
                Some(RedirectMode::Capture.into_spanned(*redir_span)),
                None,
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

fn redirect_mode_of_expression(
    engine_state: &EngineState,
    expression: &Expression,
) -> Result<(Option<RedirectMode>, Option<RedirectMode>), CompileError> {
    let (out, err) = expression.expr.pipe_redirection(&engine_state);
    Ok((
        out.map(|out| out_dest_to_redirect_mode(out)).transpose()?,
        err.map(|err| out_dest_to_redirect_mode(err)).transpose()?,
    ))
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
    engine_state: &EngineState,
    builder: &mut BlockBuilder,
    expr: &Expression,
    out_mode: Option<Spanned<RedirectMode>>,
    err_mode: Option<Spanned<RedirectMode>>,
    in_reg: Option<RegId>,
    out_reg: RegId,
) -> Result<(), CompileError> {
    let lit = |builder: &mut BlockBuilder, literal: Literal| {
        builder.push(
            Instruction::LoadLiteral {
                dst: out_reg,
                lit: literal,
            }
            .into_spanned(expr.span),
        )
    };

    match &expr.expr {
        Expr::Bool(b) => lit(builder, Literal::Bool(*b)),
        Expr::Int(i) => lit(builder, Literal::Int(*i)),
        Expr::Float(f) => lit(builder, Literal::Float(*f)),
        Expr::Binary(bin) => lit(builder, Literal::Binary(bin.as_slice().into())),
        Expr::Range(_) => todo!(),
        Expr::Var(_) => todo!(),
        Expr::VarDecl(_) => todo!(),
        Expr::Call(call) => {
            // Ensure that out_reg contains the input value, because a call only uses one register
            if let Some(in_reg) = in_reg {
                if in_reg != out_reg {
                    // Have to move in_reg to out_reg so it can be used
                    builder.push(
                        Instruction::Move {
                            dst: out_reg,
                            src: in_reg,
                        }
                        .into_spanned(call.head),
                    )?;
                }
            } else {
                // Will have to initialize out_reg with Nothing first
                builder.load_nothing(out_reg)?;
            }

            compile_call(engine_state, builder, &call, out_mode, err_mode, out_reg)
        }
        Expr::ExternalCall(_, _) => todo!(),
        Expr::Operator(_) => todo!(),
        Expr::RowCondition(_) => todo!(),
        Expr::UnaryNot(_) => todo!(),
        Expr::BinaryOp(lhs, op, rhs) => {
            if let Expr::Operator(ref operator) = op.expr {
                compile_binary_op(
                    engine_state,
                    builder,
                    &lhs,
                    operator.clone().into_spanned(op.span),
                    &rhs,
                    in_reg,
                    out_reg,
                )
            } else {
                Err(CompileError::UnsupportedOperatorExpression)
            }
        }
        Expr::Subexpression(_) => todo!(),
        Expr::Block(_) => todo!(),
        Expr::Closure(_) => todo!(),
        Expr::MatchBlock(_) => todo!(),
        Expr::List(_) => todo!(),
        Expr::Table(_) => todo!(),
        Expr::Record(_) => todo!(),
        Expr::Keyword(_) => todo!(),
        Expr::ValueWithUnit(_) => todo!(),
        Expr::DateTime(_) => todo!(),
        Expr::Filepath(_, _) => todo!(),
        Expr::Directory(_, _) => todo!(),
        Expr::GlobPattern(_, _) => todo!(),
        Expr::String(s) => lit(builder, Literal::String(s.as_str().into())),
        Expr::RawString(rs) => lit(builder, Literal::RawString(rs.as_str().into())),
        Expr::CellPath(path) => lit(builder, Literal::CellPath(Box::new(path.clone()))),
        Expr::FullCellPath(_) => todo!(),
        Expr::ImportPattern(_) => todo!(),
        Expr::Overlay(_) => todo!(),
        Expr::Signature(_) => todo!(),
        Expr::StringInterpolation(_) => todo!(),
        Expr::Nothing => todo!(),
        Expr::Garbage => todo!(),
    }
}

fn compile_call(
    engine_state: &EngineState,
    builder: &mut BlockBuilder,
    call: &Call,
    out_mode: Option<Spanned<RedirectMode>>,
    err_mode: Option<Spanned<RedirectMode>>,
    io_reg: RegId,
) -> Result<(), CompileError> {
    // It's important that we evaluate the args first before trying to set up the argument
    // state for the call.
    //
    // We could technically compile anything that isn't another call safely without worrying about
    // the argument state, but we'd have to check all of that first and it just isn't really worth
    // it.
    enum CompiledArg {
        Positional(RegId, Span),
        Named(Box<str>, Option<RegId>, Span),
        Spread(RegId, Span),
    }

    let mut compiled_args = vec![];

    for arg in &call.arguments {
        let arg_reg = arg
            .expr()
            .map(|expr| {
                let arg_reg = builder.next_register()?;

                compile_expression(
                    engine_state,
                    builder,
                    expr,
                    Some(RedirectMode::Capture.into_spanned(arg.span())),
                    None,
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
            Argument::Named((name, _, _)) => compiled_args.push(CompiledArg::Named(
                name.item.as_str().into(),
                arg_reg,
                arg.span(),
            )),
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
                builder.push(Instruction::PushPositional { src: reg }.into_spanned(span))?
            }
            CompiledArg::Named(name, Some(reg), span) => {
                builder.push(Instruction::PushNamed { name, src: reg }.into_spanned(span))?
            }
            CompiledArg::Named(name, None, span) => {
                builder.push(Instruction::PushFlag { name }.into_spanned(span))?
            }
            CompiledArg::Spread(reg, span) => {
                builder.push(Instruction::AppendRest { src: reg }.into_spanned(span))?
            }
        }
    }

    if let Some(mode) = out_mode {
        builder.push(mode.map(|mode| Instruction::RedirectOut { mode }))?;
    }

    if let Some(mode) = err_mode {
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

fn compile_binary_op(
    engine_state: &EngineState,
    builder: &mut BlockBuilder,
    lhs: &Expression,
    op: Spanned<Operator>,
    rhs: &Expression,
    in_reg: Option<RegId>, // only for $in (TODO)
    out_reg: RegId,
) -> Result<(), CompileError> {
    // If we aren't worried about clobbering in_reg, we can write straight to out_reg
    let lhs_reg = if in_reg != Some(out_reg) {
        out_reg
    } else {
        builder.next_register()?
    };
    let rhs_reg = builder.next_register()?;

    compile_expression(
        engine_state,
        builder,
        lhs,
        Some(RedirectMode::Capture.into_spanned(op.span)),
        None,
        in_reg,
        lhs_reg,
    )?;
    compile_expression(
        engine_state,
        builder,
        rhs,
        Some(RedirectMode::Capture.into_spanned(op.span)),
        None,
        in_reg,
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

/// An internal compiler error, generally means a Nushell bug rather than an issue with user error
/// since parsing and typechecking has already passed.
#[derive(Debug)]
enum CompileError {
    RegisterOverflow,
    RegisterUninitialized(RegId),
    InvalidRedirectMode,
    Garbage,
    UnsupportedOperatorExpression,
}

impl CompileError {
    fn to_shell_error(self, span: Option<Span>) -> ShellError {
        let ice = "internal compiler error: ";
        let message = match self {
            CompileError::RegisterOverflow => format!("{ice}register overflow"),
            CompileError::RegisterUninitialized(reg_id) => {
                format!("{ice}register {reg_id} is uninitialized when used, possibly reused")
            }
            CompileError::InvalidRedirectMode => {
                format!("{ice}invalid redirect mode: File should not be specified by commands")
            }
            CompileError::Garbage => {
                format!("{ice}encountered garbage, likely due to parse error")
            }
            CompileError::UnsupportedOperatorExpression => {
                format!("{ice}unsupported operator expression")
            }
        };
        ShellError::GenericError {
            error: message,
            msg: "while compiling this code".into(),
            span,
            help: Some("this is a bug, please report it at https://github.com/nushell/nushell/issues/new along with the code you were compiling if able".into()),
            inner: vec![]
        }
    }
}

/// Builds [`IrBlock`]s progressively by consuming instructions and handles register allocation.
#[derive(Debug)]
struct BlockBuilder {
    instructions: Vec<Instruction>,
    spans: Vec<Span>,
    register_allocation_state: Vec<bool>,
}

impl BlockBuilder {
    /// Starts a new block, with the first register (`%0`) allocated as input.
    fn new() -> Self {
        BlockBuilder {
            instructions: vec![],
            spans: vec![],
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

    /// Mark a register as used, so that it can be used again by something else.
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
            Err(CompileError::RegisterUninitialized(reg_id))
        }
    }

    /// Insert an instruction into the block, automatically freeing any registers consumed by the
    /// instruction.
    fn push(&mut self, instruction: Spanned<Instruction>) -> Result<(), CompileError> {
        match &instruction.item {
            Instruction::LoadLiteral { dst: _, lit: _ } => (),
            Instruction::Move { dst: _, src } => self.free_register(*src)?,
            Instruction::Clone { dst: _, src: _ } => (),
            Instruction::Collect { src_dst: _ } => (),
            Instruction::Drain { src } => self.free_register(*src)?,
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
            Instruction::BinaryOp {
                lhs_dst: _,
                op: _,
                rhs,
            } => self.free_register(*rhs)?,
            Instruction::FollowCellPath { src_dst: _, path } => self.free_register(*path)?,
            Instruction::Jump { index: _ } => (),
            Instruction::BranchIf { cond, index: _ } => self.free_register(*cond)?,
            Instruction::Return { src } => self.free_register(*src)?,
        }
        self.instructions.push(instruction.item);
        self.spans.push(instruction.span);
        Ok(())
    }

    /// Initialize a register with [`Nothing`](Literal::Nothing).
    fn load_nothing(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        self.push(
            Instruction::LoadLiteral {
                dst: reg_id,
                lit: Literal::Nothing,
            }
            .into_spanned(Span::unknown()),
        )
    }

    /// Consume the builder and produce the final [`IrBlock`].
    fn finish(self) -> IrBlock {
        IrBlock {
            instructions: self.instructions,
            spans: self.spans,
            register_count: self.register_allocation_state.len(),
        }
    }
}
