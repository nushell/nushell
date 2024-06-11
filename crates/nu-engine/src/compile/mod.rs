use nu_protocol::{
    ast::{
        Argument, Block, Call, CellPath, Expr, Expression, Operator, PathMember, Pipeline,
        PipelineRedirection, RedirectionSource, RedirectionTarget,
    },
    engine::StateWorkingSet,
    ir::{Instruction, IrBlock, Literal, RedirectMode},
    IntoSpanned, OutDest, RegId, ShellError, Span, Spanned, ENV_VARIABLE_ID,
};

const BLOCK_INPUT: RegId = RegId(0);

/// Compile Nushell pipeline abstract syntax tree (AST) to internal representation (IR) instructions
/// for evaluation.
pub fn compile(working_set: &StateWorkingSet, block: &Block) -> Result<IrBlock, ShellError> {
    let mut builder = BlockBuilder::new();

    compile_block(
        working_set,
        &mut builder,
        block,
        RedirectModes::default(),
        Some(BLOCK_INPUT),
        BLOCK_INPUT,
    )
    .map_err(|err| err.to_shell_error(block.span))?;

    // A complete block has to end with a `return`
    builder
        .push(
            Instruction::Return { src: BLOCK_INPUT }
                .into_spanned(block.span.unwrap_or(Span::unknown())),
        )
        .map_err(|err| err.to_shell_error(block.span))?;

    Ok(builder.finish())
}

#[derive(Default)]
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
        builder.load_nothing(out_reg)
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
            redirect_modes_of_expression(working_set, &next_element.expr, span)?
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
        Expr::Range(_) => Err(CompileError::Todo("Range")),
        Expr::Var(var_id) => builder.push(
            Instruction::LoadVariable {
                dst: out_reg,
                var_id: *var_id,
            }
            .into_spanned(expr.span),
        ),
        Expr::VarDecl(_) => Err(CompileError::Todo("VarDecl")),
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

            compile_call(working_set, builder, &call, redirect_modes, out_reg)
        }
        Expr::ExternalCall(_, _) => Err(CompileError::Todo("ExternalCall")),
        Expr::Operator(_) => Err(CompileError::Todo("Operator")),
        Expr::RowCondition(_) => Err(CompileError::Todo("RowCondition")),
        Expr::UnaryNot(_) => Err(CompileError::Todo("UnaryNot")),
        Expr::BinaryOp(lhs, op, rhs) => {
            if let Expr::Operator(ref operator) = op.expr {
                compile_binary_op(
                    working_set,
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
        Expr::Block(_) => Err(CompileError::Todo("Block")),
        Expr::Closure(_) => Err(CompileError::Todo("Closure")),
        Expr::MatchBlock(_) => Err(CompileError::Todo("MatchBlock")),
        Expr::List(_) => Err(CompileError::Todo("List")),
        Expr::Table(_) => Err(CompileError::Todo("Table")),
        Expr::Record(_) => Err(CompileError::Todo("Record")),
        Expr::Keyword(_) => Err(CompileError::Todo("Keyword")),
        Expr::ValueWithUnit(_) => Err(CompileError::Todo("ValueWithUnit")),
        Expr::DateTime(_) => Err(CompileError::Todo("DateTime")),
        Expr::Filepath(_, _) => Err(CompileError::Todo("Filepath")),
        Expr::Directory(_, _) => Err(CompileError::Todo("Directory")),
        Expr::GlobPattern(_, _) => Err(CompileError::Todo("GlobPattern")),
        Expr::String(s) => lit(builder, Literal::String(s.as_str().into())),
        Expr::RawString(rs) => lit(builder, Literal::RawString(rs.as_str().into())),
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
        Expr::Signature(_) => Err(CompileError::Todo("Signature")),
        Expr::StringInterpolation(_) => Err(CompileError::Todo("StringInterpolation")),
        Expr::Nothing => Err(CompileError::Todo("Nothing")),
        Expr::Garbage => Err(CompileError::Todo("Garbage")),
    }
}

fn compile_call(
    working_set: &StateWorkingSet,
    builder: &mut BlockBuilder,
    call: &Call,
    redirect_modes: RedirectModes,
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

fn compile_binary_op(
    working_set: &StateWorkingSet,
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
        working_set,
        builder,
        lhs,
        RedirectModes::capture_out(op.span),
        in_reg,
        lhs_reg,
    )?;
    compile_expression(
        working_set,
        builder,
        rhs,
        RedirectModes::capture_out(op.span),
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
        )
    } else {
        let (key, optional) = match &path[0] {
            PathMember::String { val, optional, .. } => (val.as_str().into(), *optional),
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

        Ok(())
    }
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
    AccessEnvByInt(Span),
    Todo(&'static str),
}

impl CompileError {
    fn to_shell_error(self, mut span: Option<Span>) -> ShellError {
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
            CompileError::AccessEnvByInt(local_span) => {
                span = Some(local_span);
                format!("{ice}attempted access of $env by integer path")
            }
            CompileError::Todo(msg) => {
                format!("{ice}TODO: {msg}")
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
    fn push(&mut self, instruction: Spanned<Instruction>) -> Result<(), CompileError> {
        match &instruction.item {
            Instruction::LoadLiteral { dst, lit: _ } => self.mark_register(*dst)?,
            Instruction::Move { dst, src } => {
                self.free_register(*src)?;
                self.mark_register(*dst)?;
            }
            Instruction::Clone { dst, src: _ } => self.mark_register(*dst)?,
            Instruction::Collect { src_dst: _ } => (),
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
        self.instructions.push(instruction.item);
        self.spans.push(instruction.span);
        Ok(())
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
        )
    }

    /// Allocate a new register and load a literal into it.
    fn literal(&mut self, literal: Spanned<Literal>) -> Result<RegId, CompileError> {
        let reg_id = self.next_register()?;
        self.load_literal(reg_id, literal)?;
        Ok(reg_id)
    }

    /// Initialize a register with [`Nothing`](Literal::Nothing).
    fn load_nothing(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        self.load_literal(reg_id, Literal::Nothing.into_spanned(Span::unknown()))
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
