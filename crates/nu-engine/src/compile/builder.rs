use nu_protocol::{
    CompileError, IntoSpanned, RegId, Span, Spanned,
    ast::Pattern,
    ir::{DataSlice, Instruction, IrAstRef, IrBlock, Literal},
};

/// A label identifier. Only exists while building code. Replaced with the actual target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LabelId(pub usize);

/// Builds [`IrBlock`]s progressively by consuming instructions and handles register allocation.
#[derive(Debug)]
pub(crate) struct BlockBuilder {
    pub(crate) block_span: Option<Span>,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) spans: Vec<Span>,
    /// The actual instruction index that a label refers to. While building IR, branch targets are
    /// specified as indices into this array rather than the true instruction index. This makes it
    /// easier to make modifications to code, as just this array needs to be changed, and it's also
    /// less error prone as during `finish()` we check to make sure all of the used labels have had
    /// an index actually set.
    pub(crate) labels: Vec<Option<usize>>,
    pub(crate) data: Vec<u8>,
    pub(crate) ast: Vec<Option<IrAstRef>>,
    pub(crate) comments: Vec<String>,
    pub(crate) register_allocation_state: Vec<bool>,
    pub(crate) file_count: u32,
    pub(crate) loop_stack: Vec<Loop>,
}

impl BlockBuilder {
    /// Starts a new block, with the first register (`%0`) allocated as input.
    pub(crate) fn new(block_span: Option<Span>) -> Self {
        BlockBuilder {
            block_span,
            instructions: vec![],
            spans: vec![],
            labels: vec![],
            data: vec![],
            ast: vec![],
            comments: vec![],
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
            Ok(RegId::new(index as u32))
        } else if self.register_allocation_state.len() < (u32::MAX as usize - 2) {
            let reg_id = RegId::new(self.register_allocation_state.len() as u32);
            self.register_allocation_state.push(true);
            Ok(reg_id)
        } else {
            Err(CompileError::RegisterOverflow {
                block_span: self.block_span,
            })
        }
    }

    /// Check if a register is initialized with a value.
    pub(crate) fn is_allocated(&self, reg_id: RegId) -> bool {
        self.register_allocation_state
            .get(reg_id.get() as usize)
            .is_some_and(|state| *state)
    }

    /// Mark a register as initialized.
    pub(crate) fn mark_register(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        if let Some(is_allocated) = self
            .register_allocation_state
            .get_mut(reg_id.get() as usize)
        {
            *is_allocated = true;
            Ok(())
        } else {
            Err(CompileError::RegisterOverflow {
                block_span: self.block_span,
            })
        }
    }

    /// Mark a register as empty, so that it can be used again by something else.
    #[track_caller]
    pub(crate) fn free_register(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        let index = reg_id.get() as usize;

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

    /// Define a label, which can be used by branch instructions. The target can optionally be
    /// specified now.
    pub(crate) fn label(&mut self, target_index: Option<usize>) -> LabelId {
        let label_id = self.labels.len();
        self.labels.push(target_index);
        LabelId(label_id)
    }

    /// Change the target of a label.
    pub(crate) fn set_label(
        &mut self,
        label_id: LabelId,
        target_index: usize,
    ) -> Result<(), CompileError> {
        *self
            .labels
            .get_mut(label_id.0)
            .ok_or(CompileError::UndefinedLabel {
                label_id: label_id.0,
                span: None,
            })? = Some(target_index);
        Ok(())
    }

    /// Insert an instruction into the block, automatically marking any registers populated by
    /// the instruction, and freeing any registers consumed by the instruction.
    #[track_caller]
    pub(crate) fn push(&mut self, instruction: Spanned<Instruction>) -> Result<(), CompileError> {
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
            Instruction::Span { src_dst } => allocate(&[*src_dst], &[*src_dst]),
            Instruction::Drop { src } => allocate(&[*src], &[]),
            Instruction::Drain { src } => allocate(&[*src], &[]),
            Instruction::DrainIfEnd { src } => allocate(&[*src], &[]),
            Instruction::LoadVariable { dst, var_id: _ } => allocate(&[], &[*dst]),
            Instruction::StoreVariable { var_id: _, src } => allocate(&[*src], &[]),
            Instruction::DropVariable { var_id: _ } => Ok(()),
            Instruction::LoadEnv { dst, key: _ } => allocate(&[], &[*dst]),
            Instruction::LoadEnvOpt { dst, key: _ } => allocate(&[], &[*dst]),
            Instruction::StoreEnv { key: _, src } => allocate(&[*src], &[]),
            Instruction::PushPositional { src } => allocate(&[*src], &[]),
            Instruction::AppendRest { src } => allocate(&[*src], &[]),
            Instruction::PushFlag { name: _ } => Ok(()),
            Instruction::PushShortFlag { short: _ } => Ok(()),
            Instruction::PushNamed { name: _, src } => allocate(&[*src], &[]),
            Instruction::PushShortNamed { short: _, src } => allocate(&[*src], &[]),
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
            Instruction::CheckMatchGuard { src } => allocate(&[*src], &[*src]),
            Instruction::Iterate {
                dst,
                stream,
                end_index: _,
            } => allocate(&[*stream], &[*dst, *stream]),
            Instruction::OnError { index: _ } => Ok(()),
            Instruction::OnErrorInto { index: _, dst } => allocate(&[], &[*dst]),
            Instruction::PopErrorHandler => Ok(()),
            Instruction::ReturnEarly { src } => allocate(&[*src], &[]),
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

        self.instructions.push(instruction.item);
        self.spans.push(instruction.span);
        self.ast.push(None);
        self.comments.push(String::new());
        Ok(())
    }

    /// Set the AST of the last instruction. Separate method because it's rarely used.
    pub(crate) fn set_last_ast(&mut self, ast_ref: Option<IrAstRef>) {
        *self.ast.last_mut().expect("no last instruction") = ast_ref;
    }

    /// Add a comment to the last instruction.
    pub(crate) fn add_comment(&mut self, comment: impl std::fmt::Display) {
        add_comment(
            self.comments.last_mut().expect("no last instruction"),
            comment,
        )
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
            // try using the block Span if available, since that's slightly more helpful than Span::unknown
            let span = self.block_span.unwrap_or(Span::unknown());
            self.push(Instruction::Drop { src: reg_id }.into_spanned(span))?;
        }
        Ok(())
    }

    /// Set a register to `Empty`, but mark it as in-use, e.g. for input
    pub(crate) fn load_empty(&mut self, reg_id: RegId) -> Result<(), CompileError> {
        self.drop_reg(reg_id)?;
        self.mark_register(reg_id)
    }

    /// Drain the stream in a register (fully consuming it)
    pub(crate) fn drain(&mut self, src: RegId, span: Span) -> Result<(), CompileError> {
        self.push(Instruction::Drain { src }.into_spanned(span))
    }

    /// Add data to the `data` array and return a [`DataSlice`] referencing it.
    pub(crate) fn data(&mut self, data: impl AsRef<[u8]>) -> Result<DataSlice, CompileError> {
        let data = data.as_ref();
        let start = self.data.len();
        if data.is_empty() {
            Ok(DataSlice::empty())
        } else if start + data.len() < u32::MAX as usize {
            let slice = DataSlice {
                start: start as u32,
                len: data.len() as u32,
            };
            self.data.extend_from_slice(data);
            Ok(slice)
        } else {
            Err(CompileError::DataOverflow {
                block_span: self.block_span,
            })
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
        label_id: LabelId,
        span: Span,
    ) -> Result<(), CompileError> {
        self.push(
            Instruction::BranchIf {
                cond,
                index: label_id.0,
            }
            .into_spanned(span),
        )
    }

    /// Add a `branch-if-empty` instruction
    pub(crate) fn branch_if_empty(
        &mut self,
        src: RegId,
        label_id: LabelId,
        span: Span,
    ) -> Result<(), CompileError> {
        self.push(
            Instruction::BranchIfEmpty {
                src,
                index: label_id.0,
            }
            .into_spanned(span),
        )
    }

    /// Add a `jump` instruction
    pub(crate) fn jump(&mut self, label_id: LabelId, span: Span) -> Result<(), CompileError> {
        self.push(Instruction::Jump { index: label_id.0 }.into_spanned(span))
    }

    /// Add a `match` instruction
    pub(crate) fn r#match(
        &mut self,
        pattern: Pattern,
        src: RegId,
        label_id: LabelId,
        span: Span,
    ) -> Result<(), CompileError> {
        self.push(
            Instruction::Match {
                pattern: Box::new(pattern),
                src,
                index: label_id.0,
            }
            .into_spanned(span),
        )
    }

    /// The index that the next instruction [`.push()`](Self::push)ed will have.
    pub(crate) fn here(&self) -> usize {
        self.instructions.len()
    }

    /// Allocate a new file number, for redirection.
    pub(crate) fn next_file_num(&mut self) -> Result<u32, CompileError> {
        let next = self.file_count;
        self.file_count = self
            .file_count
            .checked_add(1)
            .ok_or(CompileError::FileOverflow {
                block_span: self.block_span,
            })?;
        Ok(next)
    }

    /// Push a new loop state onto the builder. Creates new labels that must be set.
    pub(crate) fn begin_loop(&mut self) -> Loop {
        let loop_ = Loop {
            break_label: self.label(None),
            continue_label: self.label(None),
        };
        self.loop_stack.push(loop_);
        loop_
    }

    /// True if we are currently in a loop.
    pub(crate) fn is_in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }

    /// Add a loop breaking jump instruction.
    pub(crate) fn push_break(&mut self, span: Span) -> Result<(), CompileError> {
        let loop_ = self
            .loop_stack
            .last()
            .ok_or_else(|| CompileError::NotInALoop {
                msg: "`break` called from outside of a loop".into(),
                span: Some(span),
            })?;
        self.jump(loop_.break_label, span)
    }

    /// Add a loop continuing jump instruction.
    pub(crate) fn push_continue(&mut self, span: Span) -> Result<(), CompileError> {
        let loop_ = self
            .loop_stack
            .last()
            .ok_or_else(|| CompileError::NotInALoop {
                msg: "`continue` called from outside of a loop".into(),
                span: Some(span),
            })?;
        self.jump(loop_.continue_label, span)
    }

    /// Pop the loop state. Checks that the loop being ended is the same one that was expected.
    pub(crate) fn end_loop(&mut self, loop_: Loop) -> Result<(), CompileError> {
        let ended_loop = self
            .loop_stack
            .pop()
            .ok_or_else(|| CompileError::NotInALoop {
                msg: "end_loop() called outside of a loop".into(),
                span: None,
            })?;

        if ended_loop == loop_ {
            Ok(())
        } else {
            Err(CompileError::IncoherentLoopState {
                block_span: self.block_span,
            })
        }
    }

    /// Mark an unreachable code path. Produces an error at runtime if executed.
    #[allow(dead_code)] // currently unused, but might be used in the future.
    pub(crate) fn unreachable(&mut self, span: Span) -> Result<(), CompileError> {
        self.push(Instruction::Unreachable.into_spanned(span))
    }

    /// Consume the builder and produce the final [`IrBlock`].
    pub(crate) fn finish(mut self) -> Result<IrBlock, CompileError> {
        // Add comments to label targets
        for (index, label_target) in self.labels.iter().enumerate() {
            if let Some(label_target) = label_target {
                add_comment(
                    &mut self.comments[*label_target],
                    format_args!("label({index})"),
                );
            }
        }

        // Populate the actual target indices of labels into the instructions
        for ((index, instruction), span) in
            self.instructions.iter_mut().enumerate().zip(&self.spans)
        {
            if let Some(label_id) = instruction.branch_target() {
                let target_index = self.labels.get(label_id).cloned().flatten().ok_or(
                    CompileError::UndefinedLabel {
                        label_id,
                        span: Some(*span),
                    },
                )?;
                // Add a comment to the target index that we come from here
                add_comment(
                    &mut self.comments[target_index],
                    format_args!("from({index}:)"),
                );
                instruction.set_branch_target(target_index).map_err(|_| {
                    CompileError::SetBranchTargetOfNonBranchInstruction {
                        instruction: format!("{instruction:?}"),
                        span: *span,
                    }
                })?;
            }
        }

        Ok(IrBlock {
            instructions: self.instructions,
            spans: self.spans,
            data: self.data.into(),
            ast: self.ast,
            comments: self.comments.into_iter().map(|s| s.into()).collect(),
            register_count: self
                .register_allocation_state
                .len()
                .try_into()
                .expect("register count overflowed in finish() despite previous checks"),
            file_count: self.file_count,
        })
    }
}

/// Keeps track of the `break` and `continue` target labels for a loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Loop {
    pub(crate) break_label: LabelId,
    pub(crate) continue_label: LabelId,
}

/// Add a new comment to an existing one
fn add_comment(comment: &mut String, new_comment: impl std::fmt::Display) {
    use std::fmt::Write;
    write!(
        comment,
        "{}{}",
        if comment.is_empty() { "" } else { ", " },
        new_comment
    )
    .expect("formatting failed");
}
