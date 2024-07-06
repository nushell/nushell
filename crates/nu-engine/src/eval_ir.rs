use std::{fs::File, sync::Arc};

use nu_path::expand_path_with;
use nu_protocol::{
    ast::{Bits, Block, Boolean, CellPath, Comparison, Math, Operator},
    debugger::DebugContext,
    engine::{Argument, Closure, EngineState, ErrorHandler, Matcher, Redirection, Stack},
    ir::{Call, DataSlice, Instruction, IrAstRef, IrBlock, Literal, RedirectMode},
    record, DeclId, Flag, IntoPipelineData, IntoSpanned, ListStream, OutDest, PipelineData,
    PositionalArg, Range, Record, RegId, ShellError, Signature, Span, Spanned, Value, VarId,
    ENV_VARIABLE_ID,
};

use crate::{eval::is_automatic_env_var, eval_block_with_early_return};

/// Evaluate the compiled representation of a [`Block`].
pub fn eval_ir_block<D: DebugContext>(
    engine_state: &EngineState,
    stack: &mut Stack,
    block: &Block,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    // Rust does not check recursion limits outside of const evaluation.
    // But nu programs run in the same process as the shell.
    // To prevent a stack overflow in user code from crashing the shell,
    // we limit the recursion depth of function calls.
    let maximum_call_stack_depth: u64 = engine_state.config.recursion_limit as u64;
    if stack.recursion_count > maximum_call_stack_depth {
        return Err(ShellError::RecursionLimitReached {
            recursion_limit: maximum_call_stack_depth,
            span: block.span,
        });
    }

    if let Some(ir_block) = &block.ir_block {
        D::enter_block(engine_state, block);

        let args_base = stack.arguments.get_base();
        let error_handler_base = stack.error_handlers.get_base();

        // Allocate and initialize registers. I've found that it's not really worth trying to avoid
        // the heap allocation here by reusing buffers - our allocator is fast enough
        let mut registers = Vec::with_capacity(ir_block.register_count);
        let empty = std::iter::repeat_with(|| PipelineData::Empty);
        registers.extend(empty.take(ir_block.register_count));

        let result = eval_ir_block_impl::<D>(
            &mut EvalContext {
                engine_state,
                stack,
                data: &ir_block.data,
                block_span: &block.span,
                args_base,
                error_handler_base,
                redirect_out: None,
                redirect_err: None,
                matches: vec![],
                registers: &mut registers[..],
            },
            ir_block,
            input,
        );

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
    /// The span of the block
    block_span: &'a Option<Span>,
    /// Base index on the argument stack to reset to after a call
    args_base: usize,
    /// Base index on the error handler stack to reset to after a call
    error_handler_base: usize,
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
    #[inline]
    fn put_reg(&mut self, reg_id: RegId, new_value: PipelineData) {
        // log::trace!("{reg_id} <- {new_value:?}");
        self.registers[reg_id.0 as usize] = new_value;
    }

    /// Replace the contents of a register with `Empty` and then return the value that it contained
    #[inline]
    fn take_reg(&mut self, reg_id: RegId) -> PipelineData {
        // log::trace!("<- {reg_id}");
        std::mem::replace(&mut self.registers[reg_id.0 as usize], PipelineData::Empty)
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
}

/// Eval an IR block on the provided slice of registers.
fn eval_ir_block_impl<D: DebugContext>(
    ctx: &mut EvalContext<'_>,
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
        let ast = &ir_block.ast[pc];
        log::trace!(
            "{pc:-4}: {}",
            instruction.display(ctx.engine_state, ctx.data)
        );
        match eval_instruction::<D>(ctx, instruction, span, ast) {
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
        span: *ctx.block_span,
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
    ast: &Option<IrAstRef>,
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
        Instruction::PushPositional { src } => {
            let val = ctx.collect_reg(*src, *span)?;
            ctx.stack.arguments.push(Argument::Positional {
                span: *span,
                val,
                ast: ast.clone().map(|ast_ref| ast_ref.0),
            });
            Ok(Continue)
        }
        Instruction::AppendRest { src } => {
            let vals = ctx.collect_reg(*src, *span)?;
            ctx.stack.arguments.push(Argument::Spread {
                span: *span,
                vals,
                ast: ast.clone().map(|ast_ref| ast_ref.0),
            });
            Ok(Continue)
        }
        Instruction::PushFlag { name } => {
            let data = ctx.data.clone();
            ctx.stack.arguments.push(Argument::Flag {
                data,
                name: *name,
                span: *span,
            });
            Ok(Continue)
        }
        Instruction::PushNamed { name, src } => {
            let val = ctx.collect_reg(*src, *span)?;
            let data = ctx.data.clone();
            ctx.stack.arguments.push(Argument::Named {
                data,
                name: *name,
                span: *span,
                val,
                ast: ast.clone().map(|ast_ref| ast_ref.0),
            });
            Ok(Continue)
        }
        Instruction::PushParserInfo { name, info } => {
            let data = ctx.data.clone();
            ctx.stack.arguments.push(Argument::ParserInfo {
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
        Instruction::BranchIfEmpty { src, index } => {
            let data = ctx.take_reg(*src);
            let is_empty = matches!(
                data,
                PipelineData::Empty | PipelineData::Value(Value::Nothing { .. }, _)
            );
            ctx.put_reg(*src, data);

            if is_empty {
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
        Literal::Filesize(q) => Value::filesize(*q, span),
        Literal::Duration(q) => Value::duration(*q, span),
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
        Literal::Date(dt) => Value::date(**dt, span),
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
        stack: caller_stack,
        args_base,
        redirect_out,
        redirect_err,
        ..
    } = ctx;

    let args_len = caller_stack.arguments.get_len(*args_base);
    let decl = engine_state.get_decl(decl_id);

    // Set up redirect modes
    let mut caller_stack = caller_stack.push_redirection(redirect_out.take(), redirect_err.take());

    let result;

    if let Some(block_id) = decl.block_id() {
        // If the decl is a custom command
        let block = engine_state.get_block(block_id);

        // Set up a callee stack with the captures and move arguments from the stack into variables
        let mut callee_stack = caller_stack.gather_captures(engine_state, &block.captures);

        gather_arguments(
            block,
            &mut caller_stack,
            &mut callee_stack,
            *args_base,
            args_len,
            head,
        )?;

        // Add one to the recursion count, so we don't recurse too deep. Stack overflows are not
        // recoverable in Rust.
        callee_stack.recursion_count += 1;

        result = eval_block_with_early_return::<D>(engine_state, &mut callee_stack, block, input);

        // Move environment variables back into the caller stack scope if requested to do so
        if block.redirect_env {
            redirect_env(engine_state, &mut caller_stack, &mut callee_stack);
        }
    } else {
        // FIXME: precalculate this and save it somewhere
        let span = Span::merge_many(
            std::iter::once(head).chain(
                caller_stack
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
        result = decl.run(engine_state, &mut caller_stack, &(&call).into(), input);
    };

    drop(caller_stack);

    // Important that this runs, to reset state post-call:
    ctx.stack.arguments.leave_frame(ctx.args_base);
    ctx.redirect_out = None;
    ctx.redirect_err = None;

    result
}

fn find_named_var_id(sig: &Signature, name: &[u8], span: Span) -> Result<VarId, ShellError> {
    sig.named
        .iter()
        .find(|n| n.long.as_bytes() == name)
        .ok_or_else(|| ShellError::IrEvalError {
            msg: format!(
                "block does not have an argument named `{}`",
                String::from_utf8_lossy(name)
            ),
            span: Some(span),
        })
        .and_then(|flag| expect_named_var_id(flag, span))
}

fn expect_named_var_id(arg: &Flag, span: Span) -> Result<VarId, ShellError> {
    arg.var_id.ok_or_else(|| ShellError::IrEvalError {
        msg: format!(
            "block signature is missing var id for named arg `{}`",
            arg.long
        ),
        span: Some(span),
    })
}

fn expect_positional_var_id(arg: &PositionalArg, span: Span) -> Result<VarId, ShellError> {
    arg.var_id.ok_or_else(|| ShellError::IrEvalError {
        msg: format!(
            "block signature is missing var id for positional arg `{}`",
            arg.name
        ),
        span: Some(span),
    })
}

/// Move arguments from the stack into variables for a custom command
fn gather_arguments(
    block: &Block,
    caller_stack: &mut Stack,
    callee_stack: &mut Stack,
    args_base: usize,
    args_len: usize,
    call_head: Span,
) -> Result<(), ShellError> {
    let mut positional_iter = block
        .signature
        .required_positional
        .iter()
        .chain(&block.signature.optional_positional);

    // Arguments that didn't get consumed by required/optional
    let mut rest = vec![];

    for arg in caller_stack.arguments.drain_args(args_base, args_len) {
        match arg {
            Argument::Positional { span, val, .. } => {
                if let Some(positional_arg) = positional_iter.next() {
                    let var_id = expect_positional_var_id(positional_arg, span)?;
                    callee_stack.add_var(var_id, val);
                } else {
                    rest.push(val);
                }
            }
            Argument::Spread { vals, .. } => {
                if let Value::List { vals, .. } = vals {
                    rest.extend(vals);
                } else {
                    return Err(ShellError::CannotSpreadAsList { span: vals.span() });
                }
            }
            Argument::Flag { data, name, span } => {
                let var_id = find_named_var_id(&block.signature, &data[name], span)?;
                callee_stack.add_var(var_id, Value::bool(true, span))
            }
            Argument::Named {
                data,
                name,
                span,
                val,
                ..
            } => {
                let var_id = find_named_var_id(&block.signature, &data[name], span)?;
                callee_stack.add_var(var_id, val)
            }
            Argument::ParserInfo { .. } => (),
        }
    }

    // Add the collected rest of the arguments if a spread argument exists
    if let Some(rest_arg) = &block.signature.rest_positional {
        let rest_span = rest.first().map(|v| v.span()).unwrap_or(call_head);
        let var_id = expect_positional_var_id(rest_arg, rest_span)?;
        callee_stack.add_var(var_id, Value::list(rest, rest_span));
    }

    // Check for arguments that haven't yet been set and set them to their defaults
    for positional_arg in positional_iter {
        let var_id = expect_positional_var_id(positional_arg, call_head)?;
        callee_stack.add_var(
            var_id,
            positional_arg
                .default_value
                .clone()
                .unwrap_or(Value::nothing(call_head)),
        );
    }

    for named_arg in &block.signature.named {
        if let Some(var_id) = named_arg.var_id {
            // For named arguments, we do this check by looking to see if the variable was set yet on
            // the stack. This assumes that the stack's variables was previously empty, but that's a
            // fair assumption for a brand new callee stack.
            if !callee_stack.vars.iter().any(|(id, _)| *id == var_id) {
                let val = if named_arg.arg.is_none() {
                    Value::bool(false, call_head)
                } else if let Some(value) = &named_arg.default_value {
                    value.clone()
                } else {
                    Value::nothing(call_head)
                };
                callee_stack.add_var(var_id, val);
            }
        }
    }

    Ok(())
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

/// Redirect environment from the callee stack to the caller stack
fn redirect_env(engine_state: &EngineState, caller_stack: &mut Stack, callee_stack: &Stack) {
    // Grab all environment variables from the callee
    let caller_env_vars = caller_stack.get_env_var_names(engine_state);

    // remove env vars that are present in the caller but not in the callee
    // (the callee hid them)
    for var in caller_env_vars.iter() {
        if !callee_stack.has_env_var(engine_state, var) {
            caller_stack.remove_env_var(engine_state, var);
        }
    }

    // add new env vars from callee to caller
    for (var, value) in callee_stack.get_stack_env_vars() {
        caller_stack.add_env_var(var, value);
    }
}
