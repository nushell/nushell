use std::{borrow::Cow, fs::File, sync::Arc};

use nu_path::{expand_path, expand_path_with};
use nu_protocol::{
    DataSource, DeclId, ENV_VARIABLE_ID, Flag, IntoPipelineData, IntoSpanned, ListStream, OutDest,
    PipelineData, PipelineMetadata, PositionalArg, Range, Record, RegId, ShellError, Signals,
    Signature, Span, Spanned, Type, Value, VarId,
    ast::{Bits, Block, Boolean, CellPath, Comparison, Math, Operator},
    debugger::DebugContext,
    engine::{
        Argument, Closure, EngineState, ErrorHandler, Matcher, Redirection, Stack, StateWorkingSet,
    },
    ir::{Call, DataSlice, Instruction, IrAstRef, IrBlock, Literal, RedirectMode},
    shell_error::io::IoError,
};
use nu_utils::IgnoreCaseExt;

use crate::{
    ENV_CONVERSIONS, convert_env_vars, eval::is_automatic_env_var, eval_block_with_early_return,
};

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
        let mut registers = Vec::with_capacity(ir_block.register_count as usize);
        for _ in 0..ir_block.register_count {
            registers.push(PipelineData::Empty);
        }

        // Initialize file storage.
        let mut files = vec![None; ir_block.file_count as usize];

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
                files: &mut files[..],
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
    /// Intermediate pipeline data storage used by instructions, indexed by RegId
    registers: &'a mut [PipelineData],
    /// Holds open files used by redirections
    files: &'a mut [Option<Arc<File>>],
}

impl<'a> EvalContext<'a> {
    /// Replace the contents of a register with a new value
    #[inline]
    fn put_reg(&mut self, reg_id: RegId, new_value: PipelineData) {
        // log::trace!("{reg_id} <- {new_value:?}");
        self.registers[reg_id.get() as usize] = new_value;
    }

    /// Borrow the contents of a register.
    #[inline]
    fn borrow_reg(&self, reg_id: RegId) -> &PipelineData {
        &self.registers[reg_id.get() as usize]
    }

    /// Replace the contents of a register with `Empty` and then return the value that it contained
    #[inline]
    fn take_reg(&mut self, reg_id: RegId) -> PipelineData {
        // log::trace!("<- {reg_id}");
        std::mem::replace(
            &mut self.registers[reg_id.get() as usize],
            PipelineData::Empty,
        )
    }

    /// Clone data from a register. Must be collected first.
    fn clone_reg(&mut self, reg_id: RegId, error_span: Span) -> Result<PipelineData, ShellError> {
        match &self.registers[reg_id.get() as usize] {
            PipelineData::Empty => Ok(PipelineData::Empty),
            PipelineData::Value(val, meta) => Ok(PipelineData::Value(val.clone(), meta.clone())),
            _ => Err(ShellError::IrEvalError {
                msg: "Must collect to value before using instruction that clones from a register"
                    .into(),
                span: Some(error_span),
            }),
        }
    }

    /// Clone a value from a register. Must be collected first.
    fn clone_reg_value(&mut self, reg_id: RegId, fallback_span: Span) -> Result<Value, ShellError> {
        match self.clone_reg(reg_id, fallback_span)? {
            PipelineData::Empty => Ok(Value::nothing(fallback_span)),
            PipelineData::Value(val, _) => Ok(val),
            _ => unreachable!("clone_reg should never return stream data"),
        }
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
    let need_backtrace = ctx.engine_state.get_env_var("NU_BACKTRACE").is_some();

    while pc < ir_block.instructions.len() {
        let instruction = &ir_block.instructions[pc];
        let span = &ir_block.spans[pc];
        let ast = &ir_block.ast[pc];

        D::enter_instruction(ctx.engine_state, ir_block, pc, ctx.registers);

        let result = eval_instruction::<D>(ctx, instruction, span, ast, need_backtrace);

        D::leave_instruction(
            ctx.engine_state,
            ir_block,
            pc,
            ctx.registers,
            result.as_ref().err(),
        );

        match result {
            Ok(InstructionResult::Continue) => {
                pc += 1;
            }
            Ok(InstructionResult::Branch(next_pc)) => {
                pc = next_pc;
            }
            Ok(InstructionResult::Return(reg_id)) => {
                return Ok(ctx.take_reg(reg_id));
            }
            Err(
                err @ (ShellError::Return { .. }
                | ShellError::Continue { .. }
                | ShellError::Break { .. }),
            ) => {
                // These block control related errors should be passed through
                return Err(err);
            }
            Err(err) => {
                if let Some(error_handler) = ctx.stack.error_handlers.pop(ctx.error_handler_base) {
                    // If an error handler is set, branch there
                    prepare_error_handler(ctx, error_handler, Some(err.into_spanned(*span)));
                    pc = error_handler.handler_index;
                } else if need_backtrace {
                    let err = ShellError::into_chainned(err, *span);
                    return Err(err);
                } else {
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
    error: Option<Spanned<ShellError>>,
) {
    if let Some(reg_id) = error_handler.error_register {
        if let Some(error) = error {
            // Stack state has to be updated for stuff like LAST_EXIT_CODE
            ctx.stack.set_last_error(&error.item);
            // Create the error value and put it in the register
            ctx.put_reg(
                reg_id,
                error
                    .item
                    .into_value(&StateWorkingSet::new(ctx.engine_state), error.span)
                    .into_pipeline_data(),
            );
        } else {
            // Set the register to empty
            ctx.put_reg(reg_id, PipelineData::Empty);
        }
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
    need_backtrace: bool,
) -> Result<InstructionResult, ShellError> {
    use self::InstructionResult::*;

    // Check for interrupt if necessary
    instruction.check_interrupt(ctx.engine_state, span)?;

    // See the docs for `Instruction` for more information on what these instructions are supposed
    // to do.
    match instruction {
        Instruction::Unreachable => Err(ShellError::IrEvalError {
            msg: "Reached unreachable code".into(),
            span: Some(*span),
        }),
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
            let data = ctx.clone_reg(*src, *span)?;
            ctx.put_reg(*dst, data);
            Ok(Continue)
        }
        Instruction::Collect { src_dst } => {
            let data = ctx.take_reg(*src_dst);
            let value = collect(data, *span)?;
            ctx.put_reg(*src_dst, value);
            Ok(Continue)
        }
        Instruction::Span { src_dst } => {
            let data = ctx.take_reg(*src_dst);
            let spanned = data.with_span(*span);
            ctx.put_reg(*src_dst, spanned);
            Ok(Continue)
        }
        Instruction::Drop { src } => {
            ctx.take_reg(*src);
            Ok(Continue)
        }
        Instruction::Drain { src } => {
            let data = ctx.take_reg(*src);
            drain(ctx, data)
        }
        Instruction::DrainIfEnd { src } => {
            let data = ctx.take_reg(*src);
            let res = {
                let stack = &mut ctx
                    .stack
                    .push_redirection(ctx.redirect_out.clone(), ctx.redirect_err.clone());
                data.drain_to_out_dests(ctx.engine_state, stack)?
            };
            ctx.put_reg(*src, res);
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
        Instruction::DropVariable { var_id } => {
            ctx.stack.remove_var(*var_id);
            Ok(Continue)
        }
        Instruction::LoadEnv { dst, key } => {
            let key = ctx.get_str(*key, *span)?;
            if let Some(value) = get_env_var_case_insensitive(ctx, key) {
                let new_value = value.clone().into_pipeline_data();
                ctx.put_reg(*dst, new_value);
                Ok(Continue)
            } else {
                // FIXME: using the same span twice, shouldn't this really be
                // EnvVarNotFoundAtRuntime? There are tests that depend on CantFindColumn though...
                Err(ShellError::CantFindColumn {
                    col_name: key.into(),
                    span: Some(*span),
                    src_span: *span,
                })
            }
        }
        Instruction::LoadEnvOpt { dst, key } => {
            let key = ctx.get_str(*key, *span)?;
            let value = get_env_var_case_insensitive(ctx, key)
                .cloned()
                .unwrap_or(Value::nothing(*span));
            ctx.put_reg(*dst, value.into_pipeline_data());
            Ok(Continue)
        }
        Instruction::StoreEnv { key, src } => {
            let key = ctx.get_str(*key, *span)?;
            let value = ctx.collect_reg(*src, *span)?;

            let key = get_env_var_name_case_insensitive(ctx, key);

            if !is_automatic_env_var(&key) {
                let is_config = key == "config";
                let update_conversions = key == ENV_CONVERSIONS;

                ctx.stack.add_env_var(key.into_owned(), value.clone());

                if is_config {
                    ctx.stack.update_config(ctx.engine_state)?;
                }
                if update_conversions {
                    convert_env_vars(ctx.stack, ctx.engine_state, &value)?;
                }
                Ok(Continue)
            } else {
                Err(ShellError::AutomaticEnvVarSetManually {
                    envvar_name: key.into(),
                    span: *span,
                })
            }
        }
        Instruction::PushPositional { src } => {
            let val = ctx.collect_reg(*src, *span)?.with_span(*span);
            ctx.stack.arguments.push(Argument::Positional {
                span: *span,
                val,
                ast: ast.clone().map(|ast_ref| ast_ref.0),
            });
            Ok(Continue)
        }
        Instruction::AppendRest { src } => {
            let vals = ctx.collect_reg(*src, *span)?.with_span(*span);
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
                short: DataSlice::empty(),
                span: *span,
            });
            Ok(Continue)
        }
        Instruction::PushShortFlag { short } => {
            let data = ctx.data.clone();
            ctx.stack.arguments.push(Argument::Flag {
                data,
                name: DataSlice::empty(),
                short: *short,
                span: *span,
            });
            Ok(Continue)
        }
        Instruction::PushNamed { name, src } => {
            let val = ctx.collect_reg(*src, *span)?.with_span(*span);
            let data = ctx.data.clone();
            ctx.stack.arguments.push(Argument::Named {
                data,
                name: *name,
                short: DataSlice::empty(),
                span: *span,
                val,
                ast: ast.clone().map(|ast_ref| ast_ref.0),
            });
            Ok(Continue)
        }
        Instruction::PushShortNamed { short, src } => {
            let val = ctx.collect_reg(*src, *span)?.with_span(*span);
            let data = ctx.data.clone();
            ctx.stack.arguments.push(Argument::Named {
                data,
                name: DataSlice::empty(),
                short: *short,
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
            ctx.redirect_out = eval_redirection(ctx, mode, *span, RedirectionStream::Out)?;
            Ok(Continue)
        }
        Instruction::RedirectErr { mode } => {
            ctx.redirect_err = eval_redirection(ctx, mode, *span, RedirectionStream::Err)?;
            Ok(Continue)
        }
        Instruction::CheckErrRedirected { src } => match ctx.borrow_reg(*src) {
            #[cfg(feature = "os")]
            PipelineData::ByteStream(stream, _)
                if matches!(stream.source(), nu_protocol::ByteStreamSource::Child(_)) =>
            {
                Ok(Continue)
            }
            _ => Err(ShellError::GenericError {
                error: "Can't redirect stderr of internal command output".into(),
                msg: "piping stderr only works on external commands".into(),
                span: Some(*span),
                help: None,
                inner: vec![],
            }),
        },
        Instruction::OpenFile {
            file_num,
            path,
            append,
        } => {
            let path = ctx.collect_reg(*path, *span)?;
            let file = open_file(ctx, &path, *append)?;
            ctx.files[*file_num as usize] = Some(file);
            Ok(Continue)
        }
        Instruction::WriteFile { file_num, src } => {
            let src = ctx.take_reg(*src);
            let file = ctx
                .files
                .get(*file_num as usize)
                .cloned()
                .flatten()
                .ok_or_else(|| ShellError::IrEvalError {
                    msg: format!("Tried to write to file #{file_num}, but it is not open"),
                    span: Some(*span),
                })?;
            let is_external = if let PipelineData::ByteStream(stream, ..) = &src {
                stream.source().is_external()
            } else {
                false
            };
            if let Err(err) = src.write_to(file.as_ref()) {
                if is_external {
                    ctx.stack.set_last_error(&err);
                }
                Err(err)?
            } else {
                Ok(Continue)
            }
        }
        Instruction::CloseFile { file_num } => {
            if ctx.files[*file_num as usize].take().is_some() {
                Ok(Continue)
            } else {
                Err(ShellError::IrEvalError {
                    msg: format!("Tried to close file #{file_num}, but it is not open"),
                    span: Some(*span),
                })
            }
        }
        Instruction::Call { decl_id, src_dst } => {
            let input = ctx.take_reg(*src_dst);
            let mut result = eval_call::<D>(ctx, *decl_id, *span, input)?;
            if need_backtrace {
                match &mut result {
                    PipelineData::ByteStream(s, ..) => s.push_caller_span(*span),
                    PipelineData::ListStream(s, ..) => s.push_caller_span(*span),
                    _ => (),
                };
            }
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
            let glob_value = if matches!(string_value, Value::Glob { .. }) {
                // It already is a glob, so don't touch it.
                string_value
            } else {
                // Treat it as a string, then cast
                let string = string_value.into_string()?;
                Value::glob(string, *no_expand, *span)
            };
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

            let key = key.coerce_into_string()?;
            if let Some(old_value) = record.insert(&key, val) {
                return Err(ShellError::ColumnDefinedTwice {
                    col_name: key,
                    second_use: *span,
                    first_use: old_value.span(),
                });
            }

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
                if let Some(first_value) = record.insert(&key, val) {
                    return Err(ShellError::ColumnDefinedTwice {
                        col_name: key,
                        second_use: *span,
                        first_use: first_value.span(),
                    });
                }
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
                let value = data.follow_cell_path(&path.members, *span)?;
                ctx.put_reg(*src_dst, value.into_pipeline_data());
                Ok(Continue)
            } else if let PipelineData::Value(Value::Error { error, .. }, _) = path {
                Err(*error)
            } else {
                Err(ShellError::TypeMismatch {
                    err_message: "expected cell path".into(),
                    span: path.span().unwrap_or(*span),
                })
            }
        }
        Instruction::CloneCellPath { dst, src, path } => {
            let value = ctx.clone_reg_value(*src, *span)?;
            let path = ctx.take_reg(*path);
            if let PipelineData::Value(Value::CellPath { val: path, .. }, _) = path {
                let value = value.follow_cell_path(&path.members)?;
                ctx.put_reg(*dst, value.into_owned().into_pipeline_data());
                Ok(Continue)
            } else if let PipelineData::Value(Value::Error { error, .. }, _) = path {
                Err(*error)
            } else {
                Err(ShellError::TypeMismatch {
                    err_message: "expected cell path".into(),
                    span: path.span().unwrap_or(*span),
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
            } else if let PipelineData::Value(Value::Error { error, .. }, _) = path {
                Err(*error)
            } else {
                Err(ShellError::TypeMismatch {
                    err_message: "expected cell path".into(),
                    span: path.span().unwrap_or(*span),
                })
            }
        }
        Instruction::Jump { index } => Ok(Branch(*index)),
        Instruction::BranchIf { cond, index } => {
            let data = ctx.take_reg(*cond);
            let data_span = data.span();
            let val = match data {
                PipelineData::Value(Value::Bool { val, .. }, _) => val,
                PipelineData::Value(Value::Error { error, .. }, _) => {
                    return Err(*error);
                }
                _ => {
                    return Err(ShellError::TypeMismatch {
                        err_message: "expected bool".into(),
                        span: data_span.unwrap_or(*span),
                    });
                }
            };
            if val {
                Ok(Branch(*index))
            } else {
                Ok(Continue)
            }
        }
        Instruction::BranchIfEmpty { src, index } => {
            let is_empty = matches!(
                ctx.borrow_reg(*src),
                PipelineData::Empty | PipelineData::Value(Value::Nothing { .. }, _)
            );

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
            let value = ctx.clone_reg_value(*src, *span)?;
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
                Ok(Continue)
            }
        }
        Instruction::CheckMatchGuard { src } => {
            if matches!(
                ctx.borrow_reg(*src),
                PipelineData::Value(Value::Bool { .. }, _)
            ) {
                Ok(Continue)
            } else {
                Err(ShellError::MatchGuardNotBool { span: *span })
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
        Instruction::ReturnEarly { src } => {
            let val = ctx.collect_reg(*src, *span)?;
            Err(ShellError::Return {
                span: *span,
                value: Box::new(val),
            })
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
        Literal::Block(block_id) | Literal::RowCondition(block_id) | Literal::Closure(block_id) => {
            let block = ctx.engine_state.get_block(*block_id);
            let captures = block
                .captures
                .iter()
                .map(|(var_id, span)| get_var(ctx, *var_id, *span).map(|val| (*var_id, val)))
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
                let path = expand_path(path, true);
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
                let path = expand_path(path, true);
                Value::string(path.to_string_lossy(), span)
            }
        }
        Literal::GlobPattern { val, no_expand } => {
            Value::glob(ctx.get_str(*val, span)?, *no_expand, span)
        }
        Literal::String(s) => Value::string(ctx.get_str(*s, span)?, span),
        Literal::RawString(s) => Value::string(ctx.get_str(*s, span)?, span),
        Literal::CellPath(path) => Value::cell_path(CellPath::clone(path), span),
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

    // Handle binary op errors early
    if let Value::Error { error, .. } = lhs_val {
        return Err(*error);
    }
    if let Value::Error { error, .. } = rhs_val {
        return Err(*error);
    }

    // We only have access to one span here, but the generated code usually adds a `span`
    // instruction to set the output span to the right span.
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
            Comparison::Has => lhs_val.has(op_span, &rhs_val, span)?,
            Comparison::NotHas => lhs_val.not_has(op_span, &rhs_val, span)?,
            Comparison::StartsWith => lhs_val.starts_with(op_span, &rhs_val, span)?,
            Comparison::EndsWith => lhs_val.ends_with(op_span, &rhs_val, span)?,
        },
        Operator::Math(mat) => match mat {
            Math::Add => lhs_val.add(op_span, &rhs_val, span)?,
            Math::Subtract => lhs_val.sub(op_span, &rhs_val, span)?,
            Math::Multiply => lhs_val.mul(op_span, &rhs_val, span)?,
            Math::Divide => lhs_val.div(op_span, &rhs_val, span)?,
            Math::FloorDivide => lhs_val.floor_div(op_span, &rhs_val, span)?,
            Math::Modulo => lhs_val.modulo(op_span, &rhs_val, span)?,
            Math::Pow => lhs_val.pow(op_span, &rhs_val, span)?,
            Math::Concatenate => lhs_val.concat(op_span, &rhs_val, span)?,
        },
        Operator::Boolean(bl) => match bl {
            Boolean::Or => lhs_val.or(op_span, &rhs_val, span)?,
            Boolean::Xor => lhs_val.xor(op_span, &rhs_val, span)?,
            Boolean::And => lhs_val.and(op_span, &rhs_val, span)?,
        },
        Operator::Bits(bit) => match bit {
            Bits::BitOr => lhs_val.bit_or(op_span, &rhs_val, span)?,
            Bits::BitXor => lhs_val.bit_xor(op_span, &rhs_val, span)?,
            Bits::BitAnd => lhs_val.bit_and(op_span, &rhs_val, span)?,
            Bits::ShiftLeft => lhs_val.bit_shl(op_span, &rhs_val, span)?,
            Bits::ShiftRight => lhs_val.bit_shr(op_span, &rhs_val, span)?,
        },
        Operator::Assignment(_asg) => {
            return Err(ShellError::IrEvalError {
                msg: "can't eval assignment with the `binary-op` instruction".into(),
                span: Some(span),
            });
        }
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

    let result = (|| {
        if let Some(block_id) = decl.block_id() {
            // If the decl is a custom command
            let block = engine_state.get_block(block_id);

            // check types after acquiring block to avoid unnecessarily cloning Signature
            check_input_types(&input, &block.signature, head)?;

            // Set up a callee stack with the captures and move arguments from the stack into variables
            let mut callee_stack = caller_stack.gather_captures(engine_state, &block.captures);

            gather_arguments(
                engine_state,
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

            let result =
                eval_block_with_early_return::<D>(engine_state, &mut callee_stack, block, input);

            // Move environment variables back into the caller stack scope if requested to do so
            if block.redirect_env {
                redirect_env(engine_state, &mut caller_stack, &callee_stack);
            }

            result
        } else {
            check_input_types(&input, &decl.signature(), head)?;
            // FIXME: precalculate this and save it somewhere
            let span = Span::merge_many(
                std::iter::once(head).chain(
                    caller_stack
                        .arguments
                        .get_args(*args_base, args_len)
                        .iter()
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
            decl.run(engine_state, &mut caller_stack, &(&call).into(), input)
        }
    })();

    drop(caller_stack);

    // Important that this runs, to reset state post-call:
    ctx.stack.arguments.leave_frame(ctx.args_base);
    ctx.redirect_out = None;
    ctx.redirect_err = None;

    result
}

fn find_named_var_id(
    sig: &Signature,
    name: &[u8],
    short: &[u8],
    span: Span,
) -> Result<VarId, ShellError> {
    sig.named
        .iter()
        .find(|n| {
            if !n.long.is_empty() {
                n.long.as_bytes() == name
            } else {
                // It's possible to only have a short name and no long name
                n.short
                    .is_some_and(|s| s.encode_utf8(&mut [0; 4]).as_bytes() == short)
            }
        })
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
    engine_state: &EngineState,
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
        .map(|p| (p, true))
        .chain(
            block
                .signature
                .optional_positional
                .iter()
                .map(|p| (p, false)),
        );

    // Arguments that didn't get consumed by required/optional
    let mut rest = vec![];
    let mut rest_span: Option<Span> = None;

    // If we encounter a spread, all further positionals should go to rest
    let mut always_spread = false;

    for arg in caller_stack.arguments.drain_args(args_base, args_len) {
        match arg {
            Argument::Positional { span, val, .. } => {
                // Don't check next positional arg if we encountered a spread previously
                let next = (!always_spread).then(|| positional_iter.next()).flatten();
                if let Some((positional_arg, required)) = next {
                    let var_id = expect_positional_var_id(positional_arg, span)?;
                    if required {
                        // By checking the type of the bound variable rather than converting the
                        // SyntaxShape here, we might be able to save some allocations and effort
                        let variable = engine_state.get_var(var_id);
                        check_type(&val, &variable.ty)?;
                    }
                    callee_stack.add_var(var_id, val);
                } else {
                    rest_span = Some(rest_span.map_or(val.span(), |s| s.append(val.span())));
                    rest.push(val);
                }
            }
            Argument::Spread {
                vals,
                span: spread_span,
                ..
            } => {
                if let Value::List { vals, .. } = vals {
                    rest.extend(vals);
                    // Rest variable should span the spread syntax, not the list values
                    rest_span = Some(rest_span.map_or(spread_span, |s| s.append(spread_span)));
                    // All further positional args should go to spread
                    always_spread = true;
                } else if let Value::Error { error, .. } = vals {
                    return Err(*error);
                } else {
                    return Err(ShellError::CannotSpreadAsList { span: vals.span() });
                }
            }
            Argument::Flag {
                data,
                name,
                short,
                span,
            } => {
                let var_id = find_named_var_id(&block.signature, &data[name], &data[short], span)?;
                callee_stack.add_var(var_id, Value::bool(true, span))
            }
            Argument::Named {
                data,
                name,
                short,
                span,
                val,
                ..
            } => {
                let var_id = find_named_var_id(&block.signature, &data[name], &data[short], span)?;
                callee_stack.add_var(var_id, val)
            }
            Argument::ParserInfo { .. } => (),
        }
    }

    // Add the collected rest of the arguments if a spread argument exists
    if let Some(rest_arg) = &block.signature.rest_positional {
        let rest_span = rest_span.unwrap_or(call_head);
        let var_id = expect_positional_var_id(rest_arg, rest_span)?;
        callee_stack.add_var(var_id, Value::list(rest, rest_span));
    }

    // Check for arguments that haven't yet been set and set them to their defaults
    for (positional_arg, _) in positional_iter {
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

/// Type check helper. Produces `CantConvert` error if `val` is not compatible with `ty`.
fn check_type(val: &Value, ty: &Type) -> Result<(), ShellError> {
    match val {
        Value::Error { error, .. } => Err(*error.clone()),
        _ if val.is_subtype_of(ty) => Ok(()),
        _ => Err(ShellError::CantConvert {
            to_type: ty.to_string(),
            from_type: val.get_type().to_string(),
            span: val.span(),
            help: None,
        }),
    }
}

/// Type check pipeline input against command's input types
fn check_input_types(
    input: &PipelineData,
    signature: &Signature,
    head: Span,
) -> Result<(), ShellError> {
    let io_types = &signature.input_output_types;

    // If a command doesn't have any input/output types, then treat command input type as any
    if io_types.is_empty() {
        return Ok(());
    }

    // If a command only has a nothing input type, then allow any input data
    if io_types.iter().all(|(intype, _)| intype == &Type::Nothing) {
        return Ok(());
    }

    match input {
        // early return error directly if detected
        PipelineData::Value(Value::Error { error, .. }, ..) => return Err(*error.clone()),
        // bypass run-time typechecking for custom types
        PipelineData::Value(Value::Custom { .. }, ..) => return Ok(()),
        _ => (),
    }

    // Check if the input type is compatible with *any* of the command's possible input types
    if io_types
        .iter()
        .any(|(command_type, _)| input.is_subtype_of(command_type))
    {
        return Ok(());
    }

    let mut input_types = io_types
        .iter()
        .map(|(input, _)| input.to_string())
        .collect::<Vec<String>>();

    let expected_string = match input_types.len() {
        0 => {
            return Err(ShellError::NushellFailed {
                msg: "Command input type strings is empty, despite being non-zero earlier"
                    .to_string(),
            });
        }
        1 => input_types.swap_remove(0),
        2 => input_types.join(" and "),
        _ => {
            input_types
                .last_mut()
                .expect("Vector with length >2 has no elements")
                .insert_str(0, "and ");
            input_types.join(", ")
        }
    };

    match input {
        PipelineData::Empty => Err(ShellError::PipelineEmpty { dst_span: head }),
        _ => Err(ShellError::OnlySupportsThisInputType {
            exp_input_type: expected_string,
            wrong_type: input.get_type().to_string(),
            dst_span: head,
            src_span: input.span().unwrap_or(Span::unknown()),
        }),
    }
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

/// Get an environment variable, case-insensitively
fn get_env_var_case_insensitive<'a>(ctx: &'a mut EvalContext<'_>, key: &str) -> Option<&'a Value> {
    // Read scopes in order
    for overlays in ctx
        .stack
        .env_vars
        .iter()
        .rev()
        .chain(std::iter::once(&ctx.engine_state.env_vars))
    {
        // Read overlays in order
        for overlay_name in ctx.stack.active_overlays.iter().rev() {
            let Some(map) = overlays.get(overlay_name) else {
                // Skip if overlay doesn't exist in this scope
                continue;
            };
            let hidden = ctx.stack.env_hidden.get(overlay_name);
            let is_hidden = |key: &str| hidden.is_some_and(|hidden| hidden.contains(key));

            if let Some(val) = map
                // Check for exact match
                .get(key)
                // Skip when encountering an overlay where the key is hidden
                .filter(|_| !is_hidden(key))
                .or_else(|| {
                    // Check to see if it exists at all in the map, with a different case
                    map.iter().find_map(|(k, v)| {
                        // Again, skip something that's hidden
                        (k.eq_ignore_case(key) && !is_hidden(k)).then_some(v)
                    })
                })
            {
                return Some(val);
            }
        }
    }
    // Not found
    None
}

/// Get the existing name of an environment variable, case-insensitively. This is used to implement
/// case preservation of environment variables, so that changing an environment variable that
/// already exists always uses the same case.
fn get_env_var_name_case_insensitive<'a>(ctx: &mut EvalContext<'_>, key: &'a str) -> Cow<'a, str> {
    // Read scopes in order
    ctx.stack
        .env_vars
        .iter()
        .rev()
        .chain(std::iter::once(&ctx.engine_state.env_vars))
        .flat_map(|overlays| {
            // Read overlays in order
            ctx.stack
                .active_overlays
                .iter()
                .rev()
                .filter_map(|name| overlays.get(name))
        })
        .find_map(|map| {
            // Use the hashmap first to try to be faster?
            if map.contains_key(key) {
                Some(Cow::Borrowed(key))
            } else {
                map.keys().find(|k| k.eq_ignore_case(key)).map(|k| {
                    // it exists, but with a different case
                    Cow::Owned(k.to_owned())
                })
            }
        })
        // didn't exist.
        .unwrap_or(Cow::Borrowed(key))
}

/// Helper to collect values into [`PipelineData`], preserving original span and metadata
///
/// The metadata is removed if it is the file data source, as that's just meant to mark streams.
fn collect(data: PipelineData, fallback_span: Span) -> Result<PipelineData, ShellError> {
    let span = data.span().unwrap_or(fallback_span);
    let metadata = match data.metadata() {
        // Remove the `FilePath` metadata, because after `collect` it's no longer necessary to
        // check where some input came from.
        Some(PipelineMetadata {
            data_source: DataSource::FilePath(_),
            content_type: None,
        }) => None,
        other => other,
    };
    let value = data.into_value(span)?;
    Ok(PipelineData::Value(value, metadata))
}

/// Helper for drain behavior.
fn drain(ctx: &mut EvalContext<'_>, data: PipelineData) -> Result<InstructionResult, ShellError> {
    use self::InstructionResult::*;
    match data {
        PipelineData::ByteStream(stream, ..) => {
            let span = stream.span();
            let callback_spans = stream.get_caller_spans().clone();
            if let Err(mut err) = stream.drain() {
                ctx.stack.set_last_error(&err);
                if callback_spans.is_empty() {
                    return Err(err);
                } else {
                    for s in callback_spans {
                        err = ShellError::EvalBlockWithInput {
                            span: s,
                            sources: vec![err],
                        }
                    }
                    return Err(err);
                }
            } else {
                ctx.stack.set_last_exit_code(0, span);
            }
        }
        PipelineData::ListStream(stream, ..) => {
            let callback_spans = stream.get_caller_spans().clone();
            if let Err(mut err) = stream.drain() {
                if callback_spans.is_empty() {
                    return Err(err);
                } else {
                    for s in callback_spans {
                        err = ShellError::EvalBlockWithInput {
                            span: s,
                            sources: vec![err],
                        }
                    }
                    return Err(err);
                }
            }
        }
        PipelineData::Value(..) | PipelineData::Empty => {}
    }
    Ok(Continue)
}

enum RedirectionStream {
    Out,
    Err,
}

/// Open a file for redirection
fn open_file(ctx: &EvalContext<'_>, path: &Value, append: bool) -> Result<Arc<File>, ShellError> {
    let path_expanded =
        expand_path_with(path.as_str()?, ctx.engine_state.cwd(Some(ctx.stack))?, true);
    let mut options = File::options();
    if append {
        options.append(true);
    } else {
        options.write(true).truncate(true);
    }
    let file = options
        .create(true)
        .open(&path_expanded)
        .map_err(|err| IoError::new(err, path.span(), path_expanded))?;
    Ok(Arc::new(file))
}

/// Set up a [`Redirection`] from a [`RedirectMode`]
fn eval_redirection(
    ctx: &mut EvalContext<'_>,
    mode: &RedirectMode,
    span: Span,
    which: RedirectionStream,
) -> Result<Option<Redirection>, ShellError> {
    match mode {
        RedirectMode::Pipe => Ok(Some(Redirection::Pipe(OutDest::Pipe))),
        RedirectMode::PipeSeparate => Ok(Some(Redirection::Pipe(OutDest::PipeSeparate))),
        RedirectMode::Value => Ok(Some(Redirection::Pipe(OutDest::Value))),
        RedirectMode::Null => Ok(Some(Redirection::Pipe(OutDest::Null))),
        RedirectMode::Inherit => Ok(Some(Redirection::Pipe(OutDest::Inherit))),
        RedirectMode::Print => Ok(Some(Redirection::Pipe(OutDest::Print))),
        RedirectMode::File { file_num } => {
            let file = ctx
                .files
                .get(*file_num as usize)
                .cloned()
                .flatten()
                .ok_or_else(|| ShellError::IrEvalError {
                    msg: format!("Tried to redirect to file #{file_num}, but it is not open"),
                    span: Some(span),
                })?;
            Ok(Some(Redirection::File(file)))
        }
        RedirectMode::Caller => Ok(match which {
            RedirectionStream::Out => ctx.stack.pipe_stdout().cloned().map(Redirection::Pipe),
            RedirectionStream::Err => ctx.stack.pipe_stderr().cloned().map(Redirection::Pipe),
        }),
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
        if let Some(val) = list_stream.next_value() {
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
            PipelineData::ListStream(
                ListStream::new(data.into_iter(), span, Signals::EMPTY),
                metadata,
            ),
        );
        eval_iterate(ctx, dst, stream, end_index)
    }
}

/// Redirect environment from the callee stack to the caller stack
fn redirect_env(engine_state: &EngineState, caller_stack: &mut Stack, callee_stack: &Stack) {
    // TODO: make this more efficient
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

    // set config to callee config, to capture any updates to that
    caller_stack.config.clone_from(&callee_stack.config);
}
