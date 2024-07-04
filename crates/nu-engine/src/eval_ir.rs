use std::{fs::File, sync::Arc};

use nu_path::expand_path_with;
use nu_protocol::{
    ast::{Bits, Block, Boolean, CellPath, Comparison, Math, Operator},
    debugger::DebugContext,
    engine::{Argument, Closure, EngineState, ErrorHandler, Matcher, Redirection, Stack},
    ir::{Call, DataSlice, Instruction, IrBlock, Literal, RedirectMode},
    record, DeclId, IntoPipelineData, IntoSpanned, ListStream, OutDest, PipelineData, Range,
    Record, RegId, ShellError, Span, Spanned, Value, VarId, ENV_VARIABLE_ID,
};

use crate::{eval::is_automatic_env_var, eval_block_with_early_return};

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

        let args_base = stack.arguments.get_base();
        let error_handler_base = stack.error_handlers.get_base();
        let mut registers = stack.register_buf_cache.acquire(ir_block.register_count);

        let result = eval_ir_block_impl::<D>(
            &mut EvalContext {
                engine_state,
                stack,
                data: &ir_block.data,
                args_base,
                error_handler_base,
                callee_stack: None,
                redirect_out: None,
                redirect_err: None,
                matches: vec![],
                registers: &mut registers[..],
            },
            &block_span,
            ir_block,
            input,
        );

        stack.register_buf_cache.release(registers);
        stack.error_handlers.leave_frame(error_handler_base);
        stack.arguments.leave_frame(args_base);

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
    data: &'a Arc<[u8]>,
    /// Base index on the argument stack to reset to after a call
    args_base: usize,
    /// Base index on the error handler stack to reset to after a call
    error_handler_base: usize,
    /// Stack to use for callee
    callee_stack: Option<Box<Stack>>,
    /// State set by redirect-out
    redirect_out: Option<Redirection>,
    /// State set by redirect-err
    redirect_err: Option<Redirection>,
    /// Scratch space to use for `match`
    matches: Vec<(VarId, Value)>,
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
    fn collect_reg(&mut self, reg_id: RegId, fallback_span: Span) -> Result<Value, ShellError> {
        let data = self.take_reg(reg_id);
        let span = data.span().unwrap_or(fallback_span);
        data.into_value(span)
    }

    /// Get a string from data or produce evaluation error if it's invalid UTF-8
    fn get_str(&self, slice: DataSlice, error_span: Span) -> Result<&'a str, ShellError> {
        std::str::from_utf8(&self.data[slice]).map_err(|_| ShellError::IrEvalError {
            msg: format!("data slice does not refer to valid UTF-8: {slice:?}"),
            span: Some(error_span),
        })
    }

    /// Get the current stack to be used for the callee - either `callee_stack` if set, or `stack`
    fn callee_stack(&mut self) -> &mut Stack {
        self.callee_stack.as_deref_mut().unwrap_or(self.stack)
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
        log::trace!(
            "{pc:-4}: {}",
            instruction.display(ctx.engine_state, ctx.data)
        );
        match eval_instruction::<D>(ctx, instruction, span) {
            Ok(InstructionResult::Continue) => {
                pc += 1;
            }
            Ok(InstructionResult::Branch(next_pc)) => {
                pc = next_pc;
            }
            Ok(InstructionResult::Return(reg_id)) => {
                return Ok(ctx.take_reg(reg_id));
            }
            Err(err) => {
                if let Some(error_handler) = ctx.stack.error_handlers.pop(ctx.error_handler_base) {
                    // If an error handler is set, branch there
                    prepare_error_handler(ctx, error_handler, err.into_spanned(*span));
                    pc = error_handler.handler_index;
                } else {
                    // If not, exit the block with the error
                    return Err(err);
                }
            }
        }
    }

    // Fell out of the loop, without encountering a Return.
    Err(ShellError::IrEvalError {
        msg: format!(
            "Program counter out of range (pc={pc}, len={len})",
            len = ir_block.instructions.len(),
        ),
        span: block_span.clone(),
    })
}

/// Prepare the context for an error handler
fn prepare_error_handler(
    ctx: &mut EvalContext<'_>,
    error_handler: ErrorHandler,
    error: Spanned<ShellError>,
) {
    if let Some(reg_id) = error_handler.error_register {
        // Create the error value and put it in the register
        let value = Value::record(
            record! {
                "msg" => Value::string(format!("{}", error.item), error.span),
                "debug" => Value::string(format!("{:?}", error.item), error.span),
                "raw" => Value::error(error.item, error.span),
            },
            error.span,
        );
        ctx.put_reg(reg_id, PipelineData::Value(value, None));
    }
}

/// The result of performing an instruction. Describes what should happen next
#[derive(Debug)]
enum InstructionResult {
    Continue,
    Branch(usize),
    Return(RegId),
}

/// Perform an instruction
fn eval_instruction<D: DebugContext>(
    ctx: &mut EvalContext<'_>,
    instruction: &Instruction,
    span: &Span,
) -> Result<InstructionResult, ShellError> {
    use self::InstructionResult::*;

    match instruction {
        Instruction::LoadLiteral { dst, lit } => load_literal(ctx, *dst, lit, *span),
        Instruction::LoadValue { dst, val } => {
            ctx.put_reg(*dst, Value::clone(val).into_pipeline_data());
            Ok(Continue)
        }
        Instruction::Move { dst, src } => {
            let val = ctx.take_reg(*src);
            ctx.put_reg(*dst, val);
            Ok(Continue)
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
            Ok(Continue)
        }
        Instruction::Collect { src_dst } => {
            let data = ctx.take_reg(*src_dst);
            let value = collect(data, *span)?;
            ctx.put_reg(*src_dst, value);
            Ok(Continue)
        }
        Instruction::Drop { src } => {
            ctx.take_reg(*src);
            Ok(Continue)
        }
        Instruction::Drain { src } => {
            let data = ctx.take_reg(*src);
            if let Some(exit_status) = data.drain()? {
                ctx.stack.add_env_var(
                    "LAST_EXIT_CODE".into(),
                    Value::int(exit_status.code() as i64, *span),
                );
            }
            Ok(Continue)
        }
        Instruction::LoadVariable { dst, var_id } => {
            let value = get_var(ctx, *var_id, *span)?;
            ctx.put_reg(*dst, value.into_pipeline_data());
            Ok(Continue)
        }
        Instruction::StoreVariable { var_id, src } => {
            let value = ctx.collect_reg(*src, *span)?;
            ctx.stack.add_var(*var_id, value);
            Ok(Continue)
        }
        Instruction::LoadEnv { dst, key } => {
            let key = ctx.get_str(*key, *span)?;
            if let Some(value) = ctx.stack.get_env_var(ctx.engine_state, key) {
                ctx.put_reg(*dst, value.into_pipeline_data());
                Ok(Continue)
            } else {
                Err(ShellError::EnvVarNotFoundAtRuntime {
                    envvar_name: key.into(),
                    span: *span,
                })
            }
        }
        Instruction::LoadEnvOpt { dst, key } => {
            let key = ctx.get_str(*key, *span)?;
            let value = ctx
                .stack
                .get_env_var(ctx.engine_state, key)
                .unwrap_or(Value::nothing(*span));
            ctx.put_reg(*dst, value.into_pipeline_data());
            Ok(Continue)
        }
        Instruction::StoreEnv { key, src } => {
            let key = ctx.get_str(*key, *span)?;
            let value = ctx.collect_reg(*src, *span)?;
            if !is_automatic_env_var(key) {
                ctx.stack.add_env_var(key.into(), value);
                Ok(Continue)
            } else {
                Err(ShellError::AutomaticEnvVarSetManually {
                    envvar_name: key.into(),
                    span: *span,
                })
            }
        }
        Instruction::NewCalleeStack => {
            let new_stack = ctx.stack.gather_captures(ctx.engine_state, &[]);
            ctx.callee_stack = Some(Box::new(new_stack));
            Ok(Continue)
        }
        Instruction::CaptureVariable { var_id } => {
            if let Some(callee_stack) = &mut ctx.callee_stack {
                let value = ctx.stack.get_var_with_origin(*var_id, *span)?;
                callee_stack.add_var(*var_id, value);
                Ok(Continue)
            } else {
                Err(ShellError::IrEvalError {
                    msg: "capture-variable instruction without prior new-callee-stack \
                            to initialize it"
                        .into(),
                    span: Some(*span),
                })
            }
        }
        Instruction::PushVariable { var_id, src } => {
            let value = ctx.collect_reg(*src, *span)?;
            if let Some(callee_stack) = &mut ctx.callee_stack {
                callee_stack.add_var(*var_id, value);
                Ok(Continue)
            } else {
                Err(ShellError::IrEvalError {
                    msg:
                        "push-variable instruction without prior new-callee-stack to initialize it"
                            .into(),
                    span: Some(*span),
                })
            }
        }
        Instruction::PushPositional { src } => {
            let val = ctx.collect_reg(*src, *span)?;
            ctx.callee_stack()
                .arguments
                .push(Argument::Positional { span: *span, val });
            Ok(Continue)
        }
        Instruction::AppendRest { src } => {
            let vals = ctx.collect_reg(*src, *span)?;
            ctx.callee_stack()
                .arguments
                .push(Argument::Spread { span: *span, vals });
            Ok(Continue)
        }
        Instruction::PushFlag { name } => {
            let data = ctx.data.clone();
            ctx.callee_stack().arguments.push(Argument::Flag {
                data,
                name: *name,
                span: *span,
            });
            Ok(Continue)
        }
        Instruction::PushNamed { name, src } => {
            let val = ctx.collect_reg(*src, *span)?;
            let data = ctx.data.clone();
            ctx.callee_stack().arguments.push(Argument::Named {
                data,
                name: *name,
                span: *span,
                val,
            });
            Ok(Continue)
        }
        Instruction::PushParserInfo { name, info } => {
            let data = ctx.data.clone();
            ctx.callee_stack().arguments.push(Argument::ParserInfo {
                data,
                name: *name,
                info: info.clone(),
            });
            Ok(Continue)
        }
        Instruction::RedirectOut { mode } => {
            let out_dest = eval_redirection(ctx, mode, *span)?;
            ctx.redirect_out = Some(out_dest);
            Ok(Continue)
        }
        Instruction::RedirectErr { mode } => {
            let out_dest = eval_redirection(ctx, mode, *span)?;
            ctx.redirect_err = Some(out_dest);
            Ok(Continue)
        }
        Instruction::Call { decl_id, src_dst } => {
            let input = ctx.take_reg(*src_dst);
            let result = eval_call::<D>(ctx, *decl_id, *span, input)?;
            ctx.put_reg(*src_dst, result);
            Ok(Continue)
        }
        Instruction::StringAppend { src_dst, val } => {
            let string_value = ctx.collect_reg(*src_dst, *span)?;
            let operand_value = ctx.collect_reg(*val, *span)?;
            let string_span = string_value.span();

            let mut string = string_value.into_string()?;
            let operand = if let Value::String { val, .. } = operand_value {
                // Small optimization, so we don't have to copy the string *again*
                val
            } else {
                operand_value.to_expanded_string(", ", ctx.engine_state.get_config())
            };
            string.push_str(&operand);

            let new_string_value = Value::string(string, string_span);
            ctx.put_reg(*src_dst, new_string_value.into_pipeline_data());
            Ok(Continue)
        }
        Instruction::GlobFrom { src_dst, no_expand } => {
            let string_value = ctx.collect_reg(*src_dst, *span)?;
            let string = string_value.into_string()?;
            let glob_value = Value::glob(string, *no_expand, *span);
            ctx.put_reg(*src_dst, glob_value.into_pipeline_data());
            Ok(Continue)
        }
        Instruction::ListPush { src_dst, item } => {
            let list_value = ctx.collect_reg(*src_dst, *span)?;
            let item = ctx.collect_reg(*item, *span)?;
            let list_span = list_value.span();
            let mut list = list_value.into_list()?;
            list.push(item);
            ctx.put_reg(*src_dst, Value::list(list, list_span).into_pipeline_data());
            Ok(Continue)
        }
        Instruction::ListSpread { src_dst, items } => {
            let list_value = ctx.collect_reg(*src_dst, *span)?;
            let items = ctx.collect_reg(*items, *span)?;
            let list_span = list_value.span();
            let items_span = items.span();
            let mut list = list_value.into_list()?;
            list.extend(
                items
                    .into_list()
                    .map_err(|_| ShellError::CannotSpreadAsList { span: items_span })?,
            );
            ctx.put_reg(*src_dst, Value::list(list, list_span).into_pipeline_data());
            Ok(Continue)
        }
        Instruction::RecordInsert { src_dst, key, val } => {
            let record_value = ctx.collect_reg(*src_dst, *span)?;
            let key = ctx.collect_reg(*key, *span)?;
            let val = ctx.collect_reg(*val, *span)?;
            let record_span = record_value.span();
            let mut record = record_value.into_record()?;
            record.insert(key.coerce_into_string()?, val);
            ctx.put_reg(
                *src_dst,
                Value::record(record, record_span).into_pipeline_data(),
            );
            Ok(Continue)
        }
        Instruction::RecordSpread { src_dst, items } => {
            let record_value = ctx.collect_reg(*src_dst, *span)?;
            let items = ctx.collect_reg(*items, *span)?;
            let record_span = record_value.span();
            let items_span = items.span();
            let mut record = record_value.into_record()?;
            // Not using .extend() here because it doesn't handle duplicates
            for (key, val) in items
                .into_record()
                .map_err(|_| ShellError::CannotSpreadAsRecord { span: items_span })?
            {
                record.insert(key, val);
            }
            ctx.put_reg(
                *src_dst,
                Value::record(record, record_span).into_pipeline_data(),
            );
            Ok(Continue)
        }
        Instruction::Not { src_dst } => {
            let bool = ctx.collect_reg(*src_dst, *span)?;
            let negated = !bool.as_bool()?;
            ctx.put_reg(
                *src_dst,
                Value::bool(negated, bool.span()).into_pipeline_data(),
            );
            Ok(Continue)
        }
        Instruction::BinaryOp { lhs_dst, op, rhs } => binary_op(ctx, *lhs_dst, op, *rhs, *span),
        Instruction::FollowCellPath { src_dst, path } => {
            let data = ctx.take_reg(*src_dst);
            let path = ctx.take_reg(*path);
            if let PipelineData::Value(Value::CellPath { val: path, .. }, _) = path {
                let value = data.follow_cell_path(&path.members, *span, true)?;
                ctx.put_reg(*src_dst, value.into_pipeline_data());
                Ok(Continue)
            } else {
                Err(ShellError::TypeMismatch {
                    err_message: "cell path".into(),
                    span: path.span().unwrap_or(*span),
                })
            }
        }
        Instruction::CloneCellPath { dst, src, path } => {
            let data = ctx.take_reg(*src);
            let path = ctx.take_reg(*path);
            if let PipelineData::Value(value, _) = &data {
                if let PipelineData::Value(Value::CellPath { val: path, .. }, _) = path {
                    // TODO: make follow_cell_path() not have to take ownership, probably using Cow
                    let value = value.clone().follow_cell_path(&path.members, true)?;
                    ctx.put_reg(*src, data);
                    ctx.put_reg(*dst, value.into_pipeline_data());
                    Ok(Continue)
                } else {
                    Err(ShellError::TypeMismatch {
                        err_message: "cell path".into(),
                        span: path.span().unwrap_or(*span),
                    })
                }
            } else {
                Err(ShellError::IrEvalError {
                    msg: "must collect value before clone-cell-path".into(),
                    span: Some(*span),
                })
            }
        }
        Instruction::UpsertCellPath {
            src_dst,
            path,
            new_value,
        } => {
            let data = ctx.take_reg(*src_dst);
            let metadata = data.metadata();
            // Change the span because we're modifying it
            let mut value = data.into_value(*span)?;
            let path = ctx.take_reg(*path);
            let new_value = ctx.collect_reg(*new_value, *span)?;
            if let PipelineData::Value(Value::CellPath { val: path, .. }, _) = path {
                value.upsert_data_at_cell_path(&path.members, new_value)?;
                ctx.put_reg(*src_dst, value.into_pipeline_data_with_metadata(metadata));
                Ok(Continue)
            } else {
                Err(ShellError::TypeMismatch {
                    err_message: "cell path".into(),
                    span: path.span().unwrap_or(*span),
                })
            }
        }
        Instruction::Jump { index } => Ok(Branch(*index)),
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
                Ok(Branch(*index))
            } else {
                Ok(Continue)
            }
        }
        Instruction::Match {
            pattern,
            src,
            index,
        } => {
            let data = ctx.take_reg(*src);
            let PipelineData::Value(value, metadata) = data else {
                return Err(ShellError::IrEvalError {
                    msg: "must collect value before match".into(),
                    span: Some(*span),
                });
            };
            ctx.matches.clear();
            if pattern.match_value(&value, &mut ctx.matches) {
                // Match succeeded: set variables and branch
                for (var_id, match_value) in ctx.matches.drain(..) {
                    ctx.stack.add_var(var_id, match_value);
                }
                Ok(Branch(*index))
            } else {
                // Failed to match, put back original value
                ctx.matches.clear();
                ctx.put_reg(*src, PipelineData::Value(value, metadata));
                Ok(Continue)
            }
        }
        Instruction::Iterate {
            dst,
            stream,
            end_index,
        } => eval_iterate(ctx, *dst, *stream, *end_index),
        Instruction::OnError { index } => {
            ctx.stack.error_handlers.push(ErrorHandler {
                handler_index: *index,
                error_register: None,
            });
            Ok(Continue)
        }
        Instruction::OnErrorInto { index, dst } => {
            ctx.stack.error_handlers.push(ErrorHandler {
                handler_index: *index,
                error_register: Some(*dst),
            });
            Ok(Continue)
        }
        Instruction::PopErrorHandler => {
            ctx.stack.error_handlers.pop(ctx.error_handler_base);
            Ok(Continue)
        }
        Instruction::Return { src } => Ok(Return(*src)),
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
        Literal::Binary(bin) => Value::binary(&ctx.data[*bin], span),
        Literal::Block(block_id) | Literal::RowCondition(block_id) => Value::closure(
            Closure {
                block_id: *block_id,
                captures: vec![],
            },
            span,
        ),
        Literal::Closure(block_id) => {
            let block = ctx.engine_state.get_block(*block_id);
            let captures = block
                .captures
                .iter()
                .map(|var_id| get_var(ctx, *var_id, span).map(|val| (*var_id, val)))
                .collect::<Result<Vec<_>, ShellError>>()?;
            Value::closure(
                Closure {
                    block_id: *block_id,
                    captures,
                },
                span,
            )
        }
        Literal::Range {
            start,
            step,
            end,
            inclusion,
        } => {
            let start = ctx.collect_reg(*start, span)?;
            let step = ctx.collect_reg(*step, span)?;
            let end = ctx.collect_reg(*end, span)?;
            let range = Range::new(start, step, end, *inclusion, span)?;
            Value::range(range, span)
        }
        Literal::List { capacity } => Value::list(Vec::with_capacity(*capacity), span),
        Literal::Record { capacity } => Value::record(Record::with_capacity(*capacity), span),
        Literal::Filepath {
            val: path,
            no_expand,
        } => {
            let path = ctx.get_str(*path, span)?;
            if *no_expand {
                Value::string(path, span)
            } else {
                let cwd = ctx.engine_state.cwd(Some(ctx.stack))?;
                let path = expand_path_with(path, cwd, true);

                Value::string(path.to_string_lossy(), span)
            }
        }
        Literal::Directory {
            val: path,
            no_expand,
        } => {
            let path = ctx.get_str(*path, span)?;
            if path == "-" {
                Value::string("-", span)
            } else if *no_expand {
                Value::string(path, span)
            } else {
                let cwd = ctx.engine_state.cwd(Some(ctx.stack)).unwrap_or_default();
                let path = expand_path_with(path, cwd, true);

                Value::string(path.to_string_lossy(), span)
            }
        }
        Literal::GlobPattern { val, no_expand } => {
            Value::glob(ctx.get_str(*val, span)?, *no_expand, span)
        }
        Literal::String(s) => Value::string(ctx.get_str(*s, span)?, span),
        Literal::RawString(s) => Value::string(ctx.get_str(*s, span)?, span),
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
    let lhs_val = ctx.collect_reg(lhs_dst, span)?;
    let rhs_val = ctx.collect_reg(rhs, span)?;

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
fn eval_call<D: DebugContext>(
    ctx: &mut EvalContext<'_>,
    decl_id: DeclId,
    head: Span,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let EvalContext {
        engine_state,
        stack,
        args_base,
        callee_stack,
        redirect_out,
        redirect_err,
        ..
    } = ctx;

    let stack = callee_stack.as_deref_mut().unwrap_or(stack);

    let args_len = stack.arguments.get_len(*args_base);
    let decl = engine_state.get_decl(decl_id);

    // Set up redirect modes
    let mut stack = stack.push_redirection(redirect_out.take(), redirect_err.take());

    let result = if let Some(block_id) = decl.block_id() {
        // If the decl is a custom command, we assume that we have set up the arguments using
        // new-callee-stack and push-variable instead of stack.arguments
        //
        // This saves us from having to parse through the declaration at eval time to figure out
        // what to put where.
        let block = engine_state.get_block(block_id);

        eval_block_with_early_return::<D>(engine_state, &mut stack, block, input)
    } else {
        // should this be precalculated? ideally we just use the call builder...
        let span = Span::merge_many(
            std::iter::once(head).chain(
                stack
                    .arguments
                    .get_args(*args_base, args_len)
                    .into_iter()
                    .flat_map(|arg| arg.span()),
            ),
        );

        let call = Call {
            decl_id,
            head,
            span,
            args_base: *args_base,
            args_len,
        };

        // Run the call
        decl.run(engine_state, &mut stack, &(&call).into(), input)
    };

    // Important that this runs, to reset state post-call:
    stack.arguments.leave_frame(ctx.args_base);
    *redirect_out = None;
    *redirect_err = None;

    drop(stack);
    *callee_stack = None;

    result
}

/// Get variable from [`Stack`] or [`EngineState`]
fn get_var(ctx: &EvalContext<'_>, var_id: VarId, span: Span) -> Result<Value, ShellError> {
    match var_id {
        // $env
        ENV_VARIABLE_ID => {
            let env_vars = ctx.stack.get_env_vars(ctx.engine_state);
            let env_columns = env_vars.keys();
            let env_values = env_vars.values();

            let mut pairs = env_columns
                .map(|x| x.to_string())
                .zip(env_values.cloned())
                .collect::<Vec<(String, Value)>>();

            pairs.sort_by(|a, b| a.0.cmp(&b.0));

            Ok(Value::record(pairs.into_iter().collect(), span))
        }
        _ => ctx.stack.get_var(var_id, span).or_else(|err| {
            // $nu is handled by getting constant
            if let Some(const_val) = ctx.engine_state.get_constant(var_id).cloned() {
                Ok(const_val.with_span(span))
            } else {
                Err(err)
            }
        }),
    }
}

/// Helper to collect values into [`PipelineData`], preserving original span and metadata
fn collect(data: PipelineData, fallback_span: Span) -> Result<PipelineData, ShellError> {
    let span = data.span().unwrap_or(fallback_span);
    let metadata = data.metadata();
    let value = data.into_value(span)?;
    Ok(PipelineData::Value(value, metadata))
}

/// Set up a [`Redirection`] from a [`RedirectMode`]
fn eval_redirection(
    ctx: &mut EvalContext<'_>,
    mode: &RedirectMode,
    span: Span,
) -> Result<Redirection, ShellError> {
    match mode {
        RedirectMode::Pipe => Ok(Redirection::Pipe(OutDest::Pipe)),
        RedirectMode::Capture => Ok(Redirection::Pipe(OutDest::Capture)),
        RedirectMode::Null => Ok(Redirection::Pipe(OutDest::Null)),
        RedirectMode::Inherit => Ok(Redirection::Pipe(OutDest::Inherit)),
        RedirectMode::File { path, append } => {
            let path = ctx.collect_reg(*path, span)?;
            let path_expanded =
                expand_path_with(path.as_str()?, ctx.engine_state.cwd(Some(ctx.stack))?, true);
            let mut options = File::options();
            if *append {
                options.append(true);
            } else {
                options.write(true).truncate(true);
            }
            let file = options
                .create(true)
                .open(path_expanded)
                .map_err(|err| err.into_spanned(span))?;
            Ok(Redirection::File(file.into()))
        }
    }
}

/// Do an `iterate` instruction. This can be called repeatedly to get more values from an iterable
fn eval_iterate(
    ctx: &mut EvalContext<'_>,
    dst: RegId,
    stream: RegId,
    end_index: usize,
) -> Result<InstructionResult, ShellError> {
    let mut data = ctx.take_reg(stream);
    if let PipelineData::ListStream(list_stream, _) = &mut data {
        // Modify the stream, taking one value off, and branching if it's empty
        if let Some(val) = list_stream.next() {
            ctx.put_reg(dst, val.into_pipeline_data());
            ctx.put_reg(stream, data); // put the stream back so it can be iterated on again
            Ok(InstructionResult::Continue)
        } else {
            ctx.put_reg(dst, PipelineData::Empty);
            Ok(InstructionResult::Branch(end_index))
        }
    } else {
        // Convert the PipelineData to an iterator, and wrap it in a ListStream so it can be
        // iterated on
        let metadata = data.metadata();
        let span = data.span().unwrap_or(Span::unknown());
        ctx.put_reg(
            stream,
            PipelineData::ListStream(ListStream::new(data.into_iter(), span, None), metadata),
        );
        eval_iterate(ctx, dst, stream, end_index)
    }
}
