use nu_protocol::{
    ir::{DataSlice, Instruction, IrAstRef, IrBlock, Literal},
    CompileError, IntoSpanned, RegId, Span, Spanned,
};

/// Builds [`IrBlock`]s progressively by consuming instructions and handles register allocation.
#[derive(Debug)]
pub(crate) struct BlockBuilder {
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) spans: Vec<Span>,
    pub(crate) data: Vec<u8>,
    pub(crate) ast: Vec<Option<IrAstRef>>,
    pub(crate) register_allocation_state: Vec<bool>,
    pub(crate) file_count: u32,
    pub(crate) loop_stack: Vec<LoopState>,
}

impl BlockBuilder {
    /// Starts a new block, with the first register (`%0`) allocated as input.
    pub(crate) fn new() -> Self {
        BlockBuilder {
            instructions: vec![],
            spans: vec![],
            data: vec![],
            ast: vec![],
            register_allocation_state: vec![true],
            file_count: 0,
            loop_stack: vec![],
        }
    }

    /// Get the next unused register for code generation.
    pub(crate) fn next_register(&mut self) -> Result<RegId, CompileError> {
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

    /// Check if a register is initialized with a value.
    pub(crate) fn is_allocated(&self, reg_id: RegId) -> bool {
        self.register_allocation_state
            .get(reg_id.0 as usize)
            .is_some_and(|state| *state)
    }

    /// Mark a register as initialized.
    pub(crate) fn mark_register(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        if let Some(is_allocated) = self.register_allocation_state.get_mut(reg_id.0 as usize) {
            *is_allocated = true;
            Ok(())
        } else {
            Err(CompileError::RegisterOverflow)
        }
    }

    /// Mark a register as empty, so that it can be used again by something else.
    #[track_caller]
    pub(crate) fn free_register(&mut self, reg_id: RegId) -> Result<(), CompileError> {
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
            Err(CompileError::RegisterUninitialized {
                reg_id,
                caller: std::panic::Location::caller().to_string(),
            })
        }
    }

    /// Insert an instruction into the block, automatically marking any registers populated by
    /// the instruction, and freeing any registers consumed by the instruction.
    ///
    /// Returns the offset of the inserted instruction.
    #[track_caller]
    pub(crate) fn push(
        &mut self,
        instruction: Spanned<Instruction>,
    ) -> Result<usize, CompileError> {
        // Free read registers, and mark write registers.
        //
        // If a register is both read and written, it should be on both sides, so that we can verify
        // that the register was in the right state beforehand.
        let mut allocate = |read: &[RegId], write: &[RegId]| -> Result<(), CompileError> {
            for reg in read {
                self.free_register(*reg)?;
            }
            for reg in write {
                self.mark_register(*reg)?;
            }
            Ok(())
        };

        let allocate_result = match &instruction.item {
            Instruction::Unreachable => Ok(()),
            Instruction::LoadLiteral { dst, lit } => {
                allocate(&[], &[*dst]).and(
                    // Free any registers on the literal
                    match lit {
                        Literal::Range {
                            start,
                            step,
                            end,
                            inclusion: _,
                        } => allocate(&[*start, *step, *end], &[]),
                        Literal::Bool(_)
                        | Literal::Int(_)
                        | Literal::Float(_)
                        | Literal::Filesize(_)
                        | Literal::Duration(_)
                        | Literal::Binary(_)
                        | Literal::Block(_)
                        | Literal::Closure(_)
                        | Literal::RowCondition(_)
                        | Literal::List { capacity: _ }
                        | Literal::Record { capacity: _ }
                        | Literal::Filepath {
                            val: _,
                            no_expand: _,
                        }
                        | Literal::Directory {
                            val: _,
                            no_expand: _,
                        }
                        | Literal::GlobPattern {
                            val: _,
                            no_expand: _,
                        }
                        | Literal::String(_)
                        | Literal::RawString(_)
                        | Literal::CellPath(_)
                        | Literal::Date(_)
                        | Literal::Nothing => Ok(()),
                    },
                )
            }
            Instruction::LoadValue { dst, val: _ } => allocate(&[], &[*dst]),
            Instruction::Move { dst, src } => allocate(&[*src], &[*dst]),
            Instruction::Clone { dst, src } => allocate(&[*src], &[*dst, *src]),
            Instruction::Collect { src_dst } => allocate(&[*src_dst], &[*src_dst]),
            Instruction::Drop { src } => allocate(&[*src], &[]),
            Instruction::Drain { src } => allocate(&[*src], &[]),
            Instruction::LoadVariable { dst, var_id: _ } => allocate(&[], &[*dst]),
            Instruction::StoreVariable { var_id: _, src } => allocate(&[*src], &[]),
            Instruction::LoadEnv { dst, key: _ } => allocate(&[], &[*dst]),
            Instruction::LoadEnvOpt { dst, key: _ } => allocate(&[], &[*dst]),
            Instruction::StoreEnv { key: _, src } => allocate(&[*src], &[]),
            Instruction::PushPositional { src } => allocate(&[*src], &[]),
            Instruction::AppendRest { src } => allocate(&[*src], &[]),
            Instruction::PushFlag { name: _ } => Ok(()),
            Instruction::PushNamed { name: _, src } => allocate(&[*src], &[]),
            Instruction::PushParserInfo { name: _, info: _ } => Ok(()),
            Instruction::RedirectOut { mode: _ } => Ok(()),
            Instruction::RedirectErr { mode: _ } => Ok(()),
            Instruction::CheckErrRedirected { src } => allocate(&[*src], &[*src]),
            Instruction::OpenFile {
                file_num: _,
                path,
                append: _,
            } => allocate(&[*path], &[]),
            Instruction::WriteFile { file_num: _, src } => allocate(&[*src], &[]),
            Instruction::CloseFile { file_num: _ } => Ok(()),
            Instruction::Call {
                decl_id: _,
                src_dst,
            } => allocate(&[*src_dst], &[*src_dst]),
            Instruction::StringAppend { src_dst, val } => allocate(&[*src_dst, *val], &[*src_dst]),
            Instruction::GlobFrom {
                src_dst,
                no_expand: _,
            } => allocate(&[*src_dst], &[*src_dst]),
            Instruction::ListPush { src_dst, item } => allocate(&[*src_dst, *item], &[*src_dst]),
            Instruction::ListSpread { src_dst, items } => {
                allocate(&[*src_dst, *items], &[*src_dst])
            }
            Instruction::RecordInsert { src_dst, key, val } => {
                allocate(&[*src_dst, *key, *val], &[*src_dst])
            }
            Instruction::RecordSpread { src_dst, items } => {
                allocate(&[*src_dst, *items], &[*src_dst])
            }
            Instruction::Not { src_dst } => allocate(&[*src_dst], &[*src_dst]),
            Instruction::BinaryOp {
                lhs_dst,
                op: _,
                rhs,
            } => allocate(&[*lhs_dst, *rhs], &[*lhs_dst]),
            Instruction::FollowCellPath { src_dst, path } => {
                allocate(&[*src_dst, *path], &[*src_dst])
            }
            Instruction::CloneCellPath { dst, src, path } => {
                allocate(&[*src, *path], &[*src, *dst])
            }
            Instruction::UpsertCellPath {
                src_dst,
                path,
                new_value,
            } => allocate(&[*src_dst, *path, *new_value], &[*src_dst]),
            Instruction::Jump { index: _ } => Ok(()),
            Instruction::BranchIf { cond, index: _ } => allocate(&[*cond], &[]),
            Instruction::BranchIfEmpty { src, index: _ } => allocate(&[*src], &[*src]),
            Instruction::Match {
                pattern: _,
                src,
                index: _,
            } => allocate(&[*src], &[*src]),
            Instruction::Iterate {
                dst,
                stream,
                end_index: _,
            } => allocate(&[*stream], &[*dst, *stream]),
            Instruction::OnError { index: _ } => Ok(()),
            Instruction::OnErrorInto { index: _, dst } => allocate(&[], &[*dst]),
            Instruction::PopErrorHandler => Ok(()),
            Instruction::CheckExternalFailed { dst, src } => allocate(&[*src], &[*dst, *src]),
            Instruction::Return { src } => allocate(&[*src], &[]),
        };

        // Add more context to the error
        match allocate_result {
            Ok(()) => (),
            Err(CompileError::RegisterUninitialized { reg_id, caller }) => {
                return Err(CompileError::RegisterUninitializedWhilePushingInstruction {
                    reg_id,
                    caller,
                    instruction: format!("{:?}", instruction.item),
                    span: instruction.span,
                });
            }
            Err(err) => return Err(err),
        }

        let index = self.next_instruction_index();
        self.instructions.push(instruction.item);
        self.spans.push(instruction.span);
        self.ast.push(None);
        Ok(index)
    }

    /// Set the AST of the last instruction. Separate method because it's rarely used.
    pub(crate) fn set_last_ast(&mut self, ast_ref: Option<IrAstRef>) {
        *self.ast.last_mut().expect("no last instruction") = ast_ref;
    }

    /// Load a register with a literal.
    pub(crate) fn load_literal(
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
        )?;
        Ok(())
    }

    /// Allocate a new register and load a literal into it.
    pub(crate) fn literal(&mut self, literal: Spanned<Literal>) -> Result<RegId, CompileError> {
        let reg_id = self.next_register()?;
        self.load_literal(reg_id, literal)?;
        Ok(reg_id)
    }

    /// Deallocate a register and set it to `Empty`, if it is allocated
    pub(crate) fn drop_reg(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        if self.is_allocated(reg_id) {
            self.push(Instruction::Drop { src: reg_id }.into_spanned(Span::unknown()))?;
        }
        Ok(())
    }

    /// Set a register to `Empty`, but mark it as in-use, e.g. for input
    pub(crate) fn load_empty(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        self.drop_reg(reg_id)?;
        self.mark_register(reg_id)
    }

    /// Drain the stream in a register (fully consuming it)
    pub(crate) fn drain(&mut self, src: RegId, span: Span) -> Result<usize, CompileError> {
        self.push(Instruction::Drain { src }.into_spanned(span))
    }

    /// Add data to the `data` array and return a [`DataSlice`] referencing it.
    pub(crate) fn data(&mut self, data: impl AsRef<[u8]>) -> Result<DataSlice, CompileError> {
        let start = self.data.len();
        if start + data.as_ref().len() < u32::MAX as usize {
            let slice = DataSlice {
                start: start as u32,
                len: data.as_ref().len() as u32,
            };
            self.data.extend_from_slice(data.as_ref());
            Ok(slice)
        } else {
            Err(CompileError::DataOverflow)
        }
    }

    /// Clone a register with a `clone` instruction.
    pub(crate) fn clone_reg(&mut self, src: RegId, span: Span) -> Result<RegId, CompileError> {
        let dst = self.next_register()?;
        self.push(Instruction::Clone { dst, src }.into_spanned(span))?;
        Ok(dst)
    }

    /// Add a `branch-if` instruction
    pub(crate) fn branch_if(
        &mut self,
        cond: RegId,
        index: usize,
        span: Span,
    ) -> Result<usize, CompileError> {
        self.push(Instruction::BranchIf { cond, index }.into_spanned(span))
    }

    /// Add a placeholder `branch-if` instruction, which must be updated with
    /// [`.set_branch_target()`]
    pub(crate) fn branch_if_placeholder(
        &mut self,
        cond: RegId,
        span: Span,
    ) -> Result<usize, CompileError> {
        self.branch_if(cond, usize::MAX, span)
    }

    /// Add a `branch-if-empty` instruction
    pub(crate) fn branch_if_empty(
        &mut self,
        src: RegId,
        index: usize,
        span: Span,
    ) -> Result<usize, CompileError> {
        self.push(Instruction::BranchIfEmpty { src, index }.into_spanned(span))
    }

    /// Add a `jump` instruction
    pub(crate) fn jump(&mut self, index: usize, span: Span) -> Result<usize, CompileError> {
        self.push(Instruction::Jump { index }.into_spanned(span))
    }

    /// Add a placeholder `jump` instruction, which must be updated with [`.set_branch_target()`]
    pub(crate) fn jump_placeholder(&mut self, span: Span) -> Result<usize, CompileError> {
        self.jump(usize::MAX, span)
    }

    /// Modify a branching instruction's branch target `index`
    #[track_caller]
    pub(crate) fn set_branch_target(
        &mut self,
        instruction_index: usize,
        target_index: usize,
    ) -> Result<(), CompileError> {
        match self.instructions.get_mut(instruction_index) {
            Some(
                Instruction::BranchIf { index, .. }
                | Instruction::BranchIfEmpty { index, .. }
                | Instruction::Jump { index }
                | Instruction::Match { index, .. }
                | Instruction::Iterate {
                    end_index: index, ..
                }
                | Instruction::OnError { index }
                | Instruction::OnErrorInto { index, .. },
            ) => {
                *index = target_index;
                Ok(())
            }
            Some(_) => {
                let other = &self.instructions[instruction_index];

                log::warn!("set branch target failed ({instruction_index} => {target_index}), target instruction = {other:?}, builder = {self:#?}");

                Err(CompileError::SetBranchTargetOfNonBranchInstruction {
                    instruction: format!("{other:?}"),
                    span: self.spans[instruction_index],
                    caller: std::panic::Location::caller().to_string(),
                })
            }
            None => Err(CompileError::InstructionIndexOutOfRange {
                index: instruction_index,
            }),
        }
    }

    /// The index that the next instruction [`.push()`]ed will have.
    pub(crate) fn next_instruction_index(&self) -> usize {
        self.instructions.len()
    }

    /// Allocate a new file number, for redirection.
    pub(crate) fn next_file_num(&mut self) -> Result<u32, CompileError> {
        let next = self.file_count;
        self.file_count = self
            .file_count
            .checked_add(1)
            .ok_or_else(|| CompileError::FileOverflow)?;
        Ok(next)
    }

    /// Push a new loop state onto the builder.
    pub(crate) fn begin_loop(&mut self) {
        self.loop_stack.push(LoopState::new());
    }

    /// True if we are currently in a loop.
    pub(crate) fn is_in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }

    /// Add a loop breaking jump instruction.
    pub(crate) fn push_break(&mut self, span: Span) -> Result<usize, CompileError> {
        let index = self.jump_placeholder(span)?;
        self.loop_stack
            .last_mut()
            .ok_or_else(|| CompileError::NotInALoop {
                msg: "`break` called from outside of a loop".into(),
                span: Some(span),
            })?
            .break_branches
            .push(index);
        Ok(index)
    }

    /// Add a loop continuing jump instruction.
    pub(crate) fn push_continue(&mut self, span: Span) -> Result<usize, CompileError> {
        let index = self.jump_placeholder(span)?;
        self.loop_stack
            .last_mut()
            .ok_or_else(|| CompileError::NotInALoop {
                msg: "`continue` called from outside of a loop".into(),
                span: Some(span),
            })?
            .continue_branches
            .push(index);
        Ok(index)
    }

    /// Pop the loop state and set any `break` or `continue` instructions to their appropriate
    /// target instruction indexes.
    pub(crate) fn end_loop(
        &mut self,
        break_target_index: usize,
        continue_target_index: usize,
    ) -> Result<(), CompileError> {
        let loop_state = self
            .loop_stack
            .pop()
            .ok_or_else(|| CompileError::NotInALoop {
                msg: "end_loop() called outside of a loop".into(),
                span: None,
            })?;

        for break_index in loop_state.break_branches {
            self.set_branch_target(break_index, break_target_index)?;
        }
        for continue_index in loop_state.continue_branches {
            self.set_branch_target(continue_index, continue_target_index)?;
        }

        Ok(())
    }

    /// Mark an unreachable code path. Produces an error at runtime if executed.
    pub(crate) fn unreachable(&mut self, span: Span) -> Result<usize, CompileError> {
        self.push(Instruction::Unreachable.into_spanned(span))
    }

    /// Consume the builder and produce the final [`IrBlock`].
    pub(crate) fn finish(self) -> IrBlock {
        IrBlock {
            instructions: self.instructions,
            spans: self.spans,
            data: self.data.into(),
            ast: self.ast,
            register_count: self
                .register_allocation_state
                .len()
                .try_into()
                .expect("register count overflowed in finish() despite previous checks"),
            file_count: self.file_count,
        }
    }
}

/// Keeps track of `break` and `continue` branches that need to be set up after a loop is compiled.
#[derive(Debug)]
pub(crate) struct LoopState {
    break_branches: Vec<usize>,
    continue_branches: Vec<usize>,
}

impl LoopState {
    pub(crate) const fn new() -> Self {
        LoopState {
            break_branches: vec![],
            continue_branches: vec![],
        }
    }
}
