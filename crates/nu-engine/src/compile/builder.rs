use nu_protocol::{
    ir::{DataSlice, Instruction, IrBlock, Literal, RedirectMode},
    CompileError, IntoSpanned, RegId, Span, Spanned,
};

/// Builds [`IrBlock`]s progressively by consuming instructions and handles register allocation.
#[derive(Debug)]
pub(crate) struct BlockBuilder {
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) spans: Vec<Span>,
    pub(crate) data: Vec<u8>,
    pub(crate) register_allocation_state: Vec<bool>,
}

impl BlockBuilder {
    /// Starts a new block, with the first register (`%0`) allocated as input.
    pub(crate) fn new() -> Self {
        BlockBuilder {
            instructions: vec![],
            spans: vec![],
            data: vec![],
            register_allocation_state: vec![true],
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
        match &instruction.item {
            Instruction::LoadLiteral { dst, lit } => {
                self.mark_register(*dst)?;
                // Free any registers on the literal
                match lit {
                    Literal::Range {
                        start,
                        step,
                        end,
                        inclusion: _,
                    } => {
                        self.free_register(*start)?;
                        self.free_register(*step)?;
                        self.free_register(*end)?;
                    }
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
                    | Literal::Nothing => (),
                }
            }
            Instruction::LoadValue { dst, val: _ } => self.mark_register(*dst)?,
            Instruction::Move { dst, src } => {
                self.free_register(*src)?;
                self.mark_register(*dst)?;
            }
            Instruction::Clone { dst, src: _ } => self.mark_register(*dst)?,
            Instruction::Collect { src_dst: _ } => (),
            Instruction::Drop { src } => self.free_register(*src)?,
            Instruction::Drain { src } => self.free_register(*src)?,
            Instruction::LoadVariable { dst, var_id: _ } => self.mark_register(*dst)?,
            Instruction::StoreVariable { var_id: _, src } => self.free_register(*src)?,
            Instruction::LoadEnv { dst, key: _ } => self.mark_register(*dst)?,
            Instruction::LoadEnvOpt { dst, key: _ } => self.mark_register(*dst)?,
            Instruction::StoreEnv { key: _, src } => self.free_register(*src)?,
            Instruction::NewCalleeStack => (),
            Instruction::CaptureVariable { var_id: _ } => (),
            Instruction::PushVariable { var_id: _, src } => self.free_register(*src)?,
            Instruction::PushPositional { src } => self.free_register(*src)?,
            Instruction::AppendRest { src } => self.free_register(*src)?,
            Instruction::PushFlag { name: _ } => (),
            Instruction::PushNamed { name: _, src } => self.free_register(*src)?,
            Instruction::PushParserInfo { name: _, info: _ } => (),
            Instruction::RedirectOut { mode } | Instruction::RedirectErr { mode } => match mode {
                RedirectMode::File { path, .. } => self.free_register(*path)?,
                _ => (),
            },
            Instruction::Call {
                decl_id: _,
                src_dst: _,
            } => (),
            Instruction::StringAppend { src_dst: _, val } => self.free_register(*val)?,
            Instruction::GlobFrom {
                src_dst: _,
                no_expand: _,
            } => (),
            Instruction::ListPush { src_dst: _, item } => self.free_register(*item)?,
            Instruction::ListSpread { src_dst: _, items } => self.free_register(*items)?,
            Instruction::RecordInsert {
                src_dst: _,
                key,
                val,
            } => {
                self.free_register(*key)?;
                self.free_register(*val)?;
            }
            Instruction::RecordSpread { src_dst: _, items } => self.free_register(*items)?,
            Instruction::Not { src_dst: _ } => (),
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
            Instruction::Match {
                pattern: _,
                src: _,
                index: _,
            } => (),
            Instruction::Iterate {
                dst,
                stream: _,
                end_index: _,
            } => self.mark_register(*dst)?,
            Instruction::OnError { index: _ } => (),
            Instruction::OnErrorInto { index: _, dst } => self.mark_register(*dst)?,
            Instruction::PopErrorHandler => (),
            Instruction::Return { src } => self.free_register(*src)?,
        }
        let index = self.next_instruction_index();
        self.instructions.push(instruction.item);
        self.spans.push(instruction.span);
        Ok(index)
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

    /// Consume the builder and produce the final [`IrBlock`].
    pub(crate) fn finish(self) -> IrBlock {
        IrBlock {
            instructions: self.instructions,
            spans: self.spans,
            data: self.data.into(),
            register_count: self.register_allocation_state.len(),
        }
    }
}
