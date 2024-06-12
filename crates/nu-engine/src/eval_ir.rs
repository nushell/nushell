use nu_protocol::{
    ast::{Bits, Block, Boolean, CellPath, Comparison, Math, Operator},
    debugger::DebugContext,
    engine::{Closure, EngineState, Stack},
    ir::{Instruction, IrBlock, Literal},
    PipelineData, RegId, ShellError, Span, Value,
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

        // Allocate required space for registers. We prefer to allocate on the stack, but will
        // allocate on the heap if it's over the compiled maximum size
        let result = match ir_block.register_count {
            0 => eval_ir_block_static::<D, 0>(engine_state, stack, &block_span, ir_block, input),
            1 => eval_ir_block_static::<D, 1>(engine_state, stack, &block_span, ir_block, input),
            2 => eval_ir_block_static::<D, 2>(engine_state, stack, &block_span, ir_block, input),
            3 => eval_ir_block_static::<D, 3>(engine_state, stack, &block_span, ir_block, input),
            4 => eval_ir_block_static::<D, 4>(engine_state, stack, &block_span, ir_block, input),
            5 => eval_ir_block_static::<D, 5>(engine_state, stack, &block_span, ir_block, input),
            6 => eval_ir_block_static::<D, 6>(engine_state, stack, &block_span, ir_block, input),
            7 => eval_ir_block_static::<D, 7>(engine_state, stack, &block_span, ir_block, input),
            8 => eval_ir_block_static::<D, 8>(engine_state, stack, &block_span, ir_block, input),
            9 => eval_ir_block_static::<D, 9>(engine_state, stack, &block_span, ir_block, input),
            10 => eval_ir_block_static::<D, 10>(engine_state, stack, &block_span, ir_block, input),
            11 => eval_ir_block_static::<D, 11>(engine_state, stack, &block_span, ir_block, input),
            12 => eval_ir_block_static::<D, 12>(engine_state, stack, &block_span, ir_block, input),
            13 => eval_ir_block_static::<D, 13>(engine_state, stack, &block_span, ir_block, input),
            14 => eval_ir_block_static::<D, 14>(engine_state, stack, &block_span, ir_block, input),
            15 => eval_ir_block_static::<D, 15>(engine_state, stack, &block_span, ir_block, input),
            16 => eval_ir_block_static::<D, 16>(engine_state, stack, &block_span, ir_block, input),
            _ => eval_ir_block_dynamic::<D>(engine_state, stack, &block_span, ir_block, input),
        };

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

/// Eval an IR block with stack-allocated registers, the size of which must be known statically.
fn eval_ir_block_static<D: DebugContext, const N: usize>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block_span: &Option<Span>,
    ir_block: &IrBlock,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    const EMPTY: PipelineData = PipelineData::Empty;
    let mut array = [EMPTY; N];
    let mut ctx = EvalContext {
        engine_state,
        stack,
        registers: &mut array[..],
    };
    eval_ir_block_impl::<D>(&mut ctx, block_span, ir_block, input)
}

/// Eval an IR block with heap-allocated registers.
fn eval_ir_block_dynamic<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block_span: &Option<Span>,
    ir_block: &IrBlock,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let mut vec = Vec::with_capacity(ir_block.register_count);
    vec.extend(std::iter::repeat_with(|| PipelineData::Empty).take(ir_block.register_count));
    let mut ctx = EvalContext {
        engine_state,
        stack,
        registers: &mut vec[..],
    };
    eval_ir_block_impl::<D>(&mut ctx, block_span, ir_block, input)
}

/// All of the pointers necessary for evaluation
struct EvalContext<'a> {
    engine_state: &'a EngineState,
    stack: &'a mut Stack,
    registers: &'a mut [PipelineData],
}

impl<'a> EvalContext<'a> {
    /// Replace the contents of a register with a new value
    fn put_reg(&mut self, reg_id: RegId, new_value: PipelineData) -> PipelineData {
        std::mem::replace(&mut self.registers[reg_id.0 as usize], new_value)
    }

    /// Replace the contents of a register with `Empty` and then return the value that it contained
    fn take_reg(&mut self, reg_id: RegId) -> PipelineData {
        self.put_reg(reg_id, PipelineData::Empty)
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
        match do_instruction(ctx, instruction, span)? {
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
fn do_instruction(
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
        Instruction::PushPositional { src } => todo!(),
        Instruction::AppendRest { src } => todo!(),
        Instruction::PushFlag { name } => todo!(),
        Instruction::PushNamed { name, src } => todo!(),
        Instruction::RedirectOut { mode } => todo!(),
        Instruction::RedirectErr { mode } => todo!(),
        Instruction::Call { decl_id, src_dst } => todo!(),
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
