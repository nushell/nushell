use nu_protocol::{
    ast::{Bits, Block, Boolean, CellPath, Comparison, Math, Operator},
    debugger::DebugContext,
    engine::{Argument, Closure, EngineState, Stack},
    ir::{Call, Instruction, IrBlock, Literal},
    DeclId, PipelineData, RegId, ShellError, Span, Value,
};

/// Evaluate the compiled representation of a [`Block`].
pub fn eval_ir_block<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    if let Some(ir_block) = &block.ir_block {
        D::enter_block(engine_state, block);

        let block_span = block.span;

        let args_base = stack.argument_stack.get_base();
        let mut registers = stack.register_buf_cache.acquire(ir_block.register_count);

        let result = eval_ir_block_impl::<D>(
            &mut EvalContext {
                engine_state,
                stack,
                args_base,
                registers: &mut registers[..],
            },
            &block_span,
            ir_block,
            input,
        );

        stack.register_buf_cache.release(registers);

        D::leave_block(engine_state, block);

        result
    } else {
        // FIXME blocks having IR should not be optional
        Err(ShellError::GenericError {
            error: "Can't evaluate block in IR mode".into(),
            msg: "block is missing compiled representation".into(),
            span: block.span,
            help: Some("the IrBlock is probably missing due to a compilation error".into()),
            inner: vec![],
        })
    }
}

/// All of the pointers necessary for evaluation
struct EvalContext<'a> {
    engine_state: &'a EngineState,
    stack: &'a mut Stack,
    /// Base index on the argument stack to reset to after a call
    args_base: usize,
    registers: &'a mut [PipelineData],
}

impl<'a> EvalContext<'a> {
    /// Replace the contents of a register with a new value
    fn put_reg(&mut self, reg_id: RegId, new_value: PipelineData) -> PipelineData {
        log::trace!("{reg_id} = {new_value:?}");
        std::mem::replace(&mut self.registers[reg_id.0 as usize], new_value)
    }

    /// Replace the contents of a register with `Empty` and then return the value that it contained
    fn take_reg(&mut self, reg_id: RegId) -> PipelineData {
        self.put_reg(reg_id, PipelineData::Empty)
    }

    /// Take and implicitly collect a register to a value
    fn collect_reg(&mut self, reg_id: RegId) -> Result<Value, ShellError> {
        let pipeline_data = self.take_reg(reg_id);
        let span = pipeline_data.span().unwrap_or(Span::unknown());
        pipeline_data.into_value(span)
    }
}

/// Eval an IR block on the provided slice of registers.
fn eval_ir_block_impl<D: DebugContext>(
    ctx: &mut EvalContext<'_>,
    block_span: &Option<Span>,
    ir_block: &IrBlock,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    if !ctx.registers.is_empty() {
        ctx.registers[0] = input;
    }

    // Program counter, starts at zero.
    let mut pc = 0;

    while pc < ir_block.instructions.len() {
        let instruction = &ir_block.instructions[pc];
        let span = &ir_block.spans[pc];
        log::trace!("{pc:-4}: {}", instruction.display(ctx.engine_state));
        match eval_instruction(ctx, instruction, span)? {
            InstructionResult::Continue => {
                pc += 1;
            }
            InstructionResult::Branch(next_pc) => {
                pc = next_pc;
            }
            InstructionResult::Return(reg_id) => {
                return Ok(ctx.take_reg(reg_id));
            }
        }
    }

    // FIXME: change to non-generic error
    Err(ShellError::GenericError {
        error: format!(
            "Program counter out of range (pc={pc}, len={len})",
            len = ir_block.instructions.len(),
        ),
        msg: "while evaluating this block".into(),
        span: block_span.clone(),
        help: Some("this indicates a compiler bug".into()),
        inner: vec![],
    })
}

/// The result of performing an instruction. Describes what should happen next
#[derive(Debug)]
enum InstructionResult {
    Continue,
    Branch(usize),
    Return(RegId),
}

/// Perform an instruction
fn eval_instruction(
    ctx: &mut EvalContext<'_>,
    instruction: &Instruction,
    span: &Span,
) -> Result<InstructionResult, ShellError> {
    match instruction {
        Instruction::LoadLiteral { dst, lit } => load_literal(ctx, *dst, lit, *span),
        Instruction::Move { dst, src } => {
            let val = ctx.take_reg(*src);
            ctx.put_reg(*dst, val);
            Ok(InstructionResult::Continue)
        }
        Instruction::Clone { dst, src } => {
            let data1 = ctx.take_reg(*src);
            let data2 = match &data1 {
                PipelineData::Empty => PipelineData::Empty,
                PipelineData::Value(val, meta) => PipelineData::Value(val.clone(), meta.clone()),
                _ => {
                    return Err(ShellError::GenericError {
                        error: "IR error: must collect before clone if a stream is expected".into(),
                        msg: "error occurred here".into(),
                        span: Some(*span),
                        help: Some("this is a compiler bug".into()),
                        inner: vec![],
                    })
                }
            };
            ctx.put_reg(*src, data1);
            ctx.put_reg(*dst, data2);
            Ok(InstructionResult::Continue)
        }
        Instruction::Collect { src_dst } => todo!(),
        Instruction::Drain { src } => todo!(),
        Instruction::LoadVariable { dst, var_id } => todo!(),
        Instruction::StoreVariable { var_id, src } => todo!(),
        Instruction::LoadEnv { dst, key } => todo!(),
        Instruction::LoadEnvOpt { dst, key } => todo!(),
        Instruction::StoreEnv { key, src } => todo!(),
        Instruction::PushPositional { src } => {
            let val = ctx.collect_reg(*src)?;
            ctx.stack
                .argument_stack
                .push(Argument::Positional { span: *span, val });
            Ok(InstructionResult::Continue)
        }
        Instruction::AppendRest { src } => {
            let vals = ctx.collect_reg(*src)?;
            ctx.stack
                .argument_stack
                .push(Argument::Spread { span: *span, vals });
            Ok(InstructionResult::Continue)
        }
        Instruction::PushFlag { name } => {
            ctx.stack.argument_stack.push(Argument::Flag {
                name: name.clone(),
                span: *span,
            });
            Ok(InstructionResult::Continue)
        }
        Instruction::PushNamed { name, src } => {
            let val = ctx.collect_reg(*src)?;
            ctx.stack.argument_stack.push(Argument::Named {
                name: name.clone(),
                span: *span,
                val,
            });
            Ok(InstructionResult::Continue)
        }
        Instruction::RedirectOut { mode } => {
            log::warn!("TODO: RedirectOut");
            Ok(InstructionResult::Continue)
        }
        Instruction::RedirectErr { mode } => {
            log::warn!("TODO: RedirectErr");
            Ok(InstructionResult::Continue)
        }
        Instruction::Call { decl_id, src_dst } => {
            let input = ctx.take_reg(*src_dst);
            let result = eval_call(ctx, *decl_id, *span, input)?;
            ctx.put_reg(*src_dst, result);
            Ok(InstructionResult::Continue)
        }
        Instruction::BinaryOp { lhs_dst, op, rhs } => binary_op(ctx, *lhs_dst, op, *rhs, *span),
        Instruction::FollowCellPath { src_dst, path } => todo!(),
        Instruction::CloneCellPath { dst, src, path } => todo!(),
        Instruction::UpsertCellPath {
            src_dst,
            path,
            new_value,
        } => todo!(),
        Instruction::Jump { index } => Ok(InstructionResult::Branch(*index)),
        Instruction::BranchIf { cond, index } => {
            let data = ctx.take_reg(*cond);
            let data_span = data.span();
            let PipelineData::Value(Value::Bool { val, .. }, _) = data else {
                return Err(ShellError::TypeMismatch {
                    err_message: "expected bool".into(),
                    span: data_span.unwrap_or(*span),
                });
            };
            if val {
                Ok(InstructionResult::Branch(*index))
            } else {
                Ok(InstructionResult::Continue)
            }
        }
        Instruction::Return { src } => Ok(InstructionResult::Return(*src)),
    }
}

/// Load a literal value into a register
fn load_literal(
    ctx: &mut EvalContext<'_>,
    dst: RegId,
    lit: &Literal,
    span: Span,
) -> Result<InstructionResult, ShellError> {
    let value = literal_value(ctx, lit, span)?;
    ctx.put_reg(dst, PipelineData::Value(value, None));
    Ok(InstructionResult::Continue)
}

fn literal_value(
    ctx: &mut EvalContext<'_>,
    lit: &Literal,
    span: Span,
) -> Result<Value, ShellError> {
    Ok(match lit {
        Literal::Bool(b) => Value::bool(*b, span),
        Literal::Int(i) => Value::int(*i, span),
        Literal::Float(f) => Value::float(*f, span),
        Literal::Binary(bin) => Value::binary(bin.clone(), span),
        // FIXME: should really represent as `Value::Closure`?
        Literal::Block(block_id) => Value::closure(
            Closure {
                block_id: *block_id,
                captures: vec![],
            },
            span,
        ),
        // TODO: look up the block and get the captures
        Literal::Closure(_block_id) => todo!(),
        Literal::List(literals) => {
            let mut vec = Vec::with_capacity(literals.len());
            for elem in literals.iter() {
                vec.push(literal_value(ctx, &elem.item, elem.span)?);
            }
            Value::list(vec, span)
        }
        Literal::Filepath { val, no_expand } => todo!(),
        Literal::Directory { val, no_expand } => todo!(),
        Literal::GlobPattern { val, no_expand } => todo!(),
        Literal::String(s) => Value::string(s.clone(), span),
        Literal::RawString(s) => Value::string(s.clone(), span),
        Literal::CellPath(path) => Value::cell_path(CellPath::clone(&path), span),
        Literal::Nothing => Value::nothing(span),
    })
}

fn binary_op(
    ctx: &mut EvalContext<'_>,
    lhs_dst: RegId,
    op: &Operator,
    rhs: RegId,
    span: Span,
) -> Result<InstructionResult, ShellError> {
    let lhs_data = ctx.take_reg(lhs_dst);
    let rhs_data = ctx.take_reg(rhs);
    let lhs_span = lhs_data.span().unwrap_or(span);
    let rhs_span = rhs_data.span().unwrap_or(span);
    let lhs_val = lhs_data.into_value(lhs_span)?;
    let rhs_val = rhs_data.into_value(rhs_span)?;

    // FIXME: there should be a span for both the operator and for the expr?
    let op_span = span;

    let result = match op {
        Operator::Comparison(cmp) => match cmp {
            Comparison::Equal => lhs_val.eq(op_span, &rhs_val, span)?,
            Comparison::NotEqual => lhs_val.ne(op_span, &rhs_val, span)?,
            Comparison::LessThan => lhs_val.lt(op_span, &rhs_val, span)?,
            Comparison::GreaterThan => lhs_val.gt(op_span, &rhs_val, span)?,
            Comparison::LessThanOrEqual => lhs_val.lte(op_span, &rhs_val, span)?,
            Comparison::GreaterThanOrEqual => lhs_val.gte(op_span, &rhs_val, span)?,
            Comparison::RegexMatch => {
                lhs_val.regex_match(ctx.engine_state, op_span, &rhs_val, false, span)?
            }
            Comparison::NotRegexMatch => {
                lhs_val.regex_match(ctx.engine_state, op_span, &rhs_val, true, span)?
            }
            Comparison::In => lhs_val.r#in(op_span, &rhs_val, span)?,
            Comparison::NotIn => lhs_val.not_in(op_span, &rhs_val, span)?,
            Comparison::StartsWith => lhs_val.starts_with(op_span, &rhs_val, span)?,
            Comparison::EndsWith => lhs_val.ends_with(op_span, &rhs_val, span)?,
        },
        Operator::Math(mat) => match mat {
            Math::Plus => lhs_val.add(op_span, &rhs_val, span)?,
            Math::Append => lhs_val.append(op_span, &rhs_val, span)?,
            Math::Minus => lhs_val.sub(op_span, &rhs_val, span)?,
            Math::Multiply => lhs_val.mul(op_span, &rhs_val, span)?,
            Math::Divide => lhs_val.div(op_span, &rhs_val, span)?,
            Math::Modulo => lhs_val.modulo(op_span, &rhs_val, span)?,
            Math::FloorDivision => lhs_val.floor_div(op_span, &rhs_val, span)?,
            Math::Pow => lhs_val.pow(op_span, &rhs_val, span)?,
        },
        Operator::Boolean(bl) => match bl {
            Boolean::And => lhs_val.and(op_span, &rhs_val, span)?,
            Boolean::Or => lhs_val.or(op_span, &rhs_val, span)?,
            Boolean::Xor => lhs_val.xor(op_span, &rhs_val, span)?,
        },
        Operator::Bits(bit) => match bit {
            Bits::BitOr => lhs_val.bit_or(op_span, &rhs_val, span)?,
            Bits::BitXor => lhs_val.bit_xor(op_span, &rhs_val, span)?,
            Bits::BitAnd => lhs_val.bit_and(op_span, &rhs_val, span)?,
            Bits::ShiftLeft => lhs_val.bit_shl(op_span, &rhs_val, span)?,
            Bits::ShiftRight => lhs_val.bit_shr(op_span, &rhs_val, span)?,
        },
        // FIXME: assignments probably shouldn't be implemented here, so this should be an error
        Operator::Assignment(_asg) => todo!(),
    };

    ctx.put_reg(lhs_dst, PipelineData::Value(result, None));

    Ok(InstructionResult::Continue)
}

/// Evaluate a call
fn eval_call(
    ctx: &mut EvalContext<'_>,
    decl_id: DeclId,
    head: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    // TODO: handle block eval
    let args_len = ctx.stack.argument_stack.get_len(ctx.args_base);
    let decl = ctx.engine_state.get_decl(decl_id);
    // should this be precalculated? ideally we just use the call builder...
    let span = ctx
        .stack
        .argument_stack
        .get_args(ctx.args_base, args_len)
        .into_iter()
        .fold(head, |span, arg| span.append(arg.span()));
    let call = Call {
        decl_id,
        head,
        span,
        args_base: ctx.args_base,
        args_len,
    };
    let result = decl.run(ctx.engine_state, ctx.stack, &(&call).into(), input);
    // Important that this runs:
    ctx.stack.argument_stack.leave_frame(ctx.args_base);
    result
}
