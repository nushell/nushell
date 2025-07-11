use crate::{
    BlockId, DeclId, Filesize, RegId, ShellError, Span, Value, VarId,
    ast::{CellPath, Expression, Operator, Pattern, RangeInclusion},
    engine::EngineState,
};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};

mod call;
mod display;

pub use call::*;
pub use display::{FmtInstruction, FmtIrBlock};

#[derive(Clone, Serialize, Deserialize)]
pub struct IrBlock {
    pub instructions: Vec<Instruction>,
    pub spans: Vec<Span>,
    #[serde(with = "serde_arc_u8_array")]
    pub data: Arc<[u8]>,
    pub ast: Vec<Option<IrAstRef>>,
    /// Additional information that can be added to help with debugging
    pub comments: Vec<Box<str>>,
    pub register_count: u32,
    pub file_count: u32,
}

impl fmt::Debug for IrBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // the ast field is too verbose and doesn't add much
        f.debug_struct("IrBlock")
            .field("instructions", &self.instructions)
            .field("spans", &self.spans)
            .field("data", &self.data)
            .field("comments", &self.comments)
            .field("register_count", &self.register_count)
            .field("file_count", &self.file_count)
            .finish_non_exhaustive()
    }
}

impl IrBlock {
    /// Returns a value that can be formatted with [`Display`](std::fmt::Display) to show a detailed
    /// listing of the instructions contained within this [`IrBlock`].
    pub fn display<'a>(&'a self, engine_state: &'a EngineState) -> FmtIrBlock<'a> {
        FmtIrBlock {
            engine_state,
            ir_block: self,
        }
    }
}

/// A slice into the `data` array of a block. This is a compact and cache-friendly way to store
/// string data that a block uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DataSlice {
    pub start: u32,
    pub len: u32,
}

impl DataSlice {
    /// A data slice that contains no data. This slice is always valid.
    pub const fn empty() -> DataSlice {
        DataSlice { start: 0, len: 0 }
    }
}

impl std::ops::Index<DataSlice> for [u8] {
    type Output = [u8];

    fn index(&self, index: DataSlice) -> &Self::Output {
        &self[index.start as usize..(index.start as usize + index.len as usize)]
    }
}

/// A possible reference into the abstract syntax tree for an instruction. This is not present for
/// most instructions and is just added when needed.
#[derive(Debug, Clone)]
pub struct IrAstRef(pub Arc<Expression>);

impl Serialize for IrAstRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IrAstRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Expression::deserialize(deserializer).map(|expr| IrAstRef(Arc::new(expr)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Unreachable code path (error)
    Unreachable,
    /// Load a literal value into the `dst` register
    LoadLiteral { dst: RegId, lit: Literal },
    /// Load a clone of a boxed value into the `dst` register (e.g. from const evaluation)
    LoadValue { dst: RegId, val: Box<Value> },
    /// Move a register. Value is taken from `src` (used by this instruction).
    Move { dst: RegId, src: RegId },
    /// Copy a register (must be a collected value). Value is still in `src` after this instruction.
    Clone { dst: RegId, src: RegId },
    /// Collect a stream in a register to a value
    Collect { src_dst: RegId },
    /// Change the span of the contents of a register to the span of this instruction.
    Span { src_dst: RegId },
    /// Drop the value/stream in a register, without draining
    Drop { src: RegId },
    /// Drain the value/stream in a register and discard (e.g. semicolon).
    ///
    /// If passed a stream from an external command, sets $env.LAST_EXIT_CODE to the resulting exit
    /// code, and invokes any available error handler with Empty, or if not available, returns an
    /// exit-code-only stream, leaving the block.
    Drain { src: RegId },
    /// Drain the value/stream in a register and discard only if this is the last pipeline element.
    // TODO: see if it's possible to remove this
    DrainIfEnd { src: RegId },
    /// Load the value of a variable into the `dst` register
    LoadVariable { dst: RegId, var_id: VarId },
    /// Store the value of a variable from the `src` register
    StoreVariable { var_id: VarId, src: RegId },
    /// Remove a variable from the stack, freeing up whatever resources were associated with it
    DropVariable { var_id: VarId },
    /// Load the value of an environment variable into the `dst` register
    LoadEnv { dst: RegId, key: DataSlice },
    /// Load the value of an environment variable into the `dst` register, or `Nothing` if it
    /// doesn't exist
    LoadEnvOpt { dst: RegId, key: DataSlice },
    /// Store the value of an environment variable from the `src` register
    StoreEnv { key: DataSlice, src: RegId },
    /// Add a positional arg to the next (internal) call.
    PushPositional { src: RegId },
    /// Add a list of args to the next (internal) call (spread/rest).
    AppendRest { src: RegId },
    /// Add a named arg with no value to the next (internal) call.
    PushFlag { name: DataSlice },
    /// Add a short named arg with no value to the next (internal) call.
    PushShortFlag { short: DataSlice },
    /// Add a named arg with a value to the next (internal) call.
    PushNamed { name: DataSlice, src: RegId },
    /// Add a short named arg with a value to the next (internal) call.
    PushShortNamed { short: DataSlice, src: RegId },
    /// Add parser info to the next (internal) call.
    PushParserInfo {
        name: DataSlice,
        info: Box<Expression>,
    },
    /// Set the redirection for stdout for the next call (only).
    ///
    /// The register for a file redirection is not consumed.
    RedirectOut { mode: RedirectMode },
    /// Set the redirection for stderr for the next call (only).
    ///
    /// The register for a file redirection is not consumed.
    RedirectErr { mode: RedirectMode },
    /// Throw an error if stderr wasn't redirected in the given stream. `src` is preserved.
    CheckErrRedirected { src: RegId },
    /// Open a file for redirection, pushing it onto the file stack.
    OpenFile {
        file_num: u32,
        path: RegId,
        append: bool,
    },
    /// Write data from the register to a file. This is done to finish a file redirection, in case
    /// an internal command or expression was evaluated rather than an external one.
    WriteFile { file_num: u32, src: RegId },
    /// Pop a file used for redirection from the file stack.
    CloseFile { file_num: u32 },
    /// Make a call. The input is taken from `src_dst`, and the output is placed in `src_dst`,
    /// overwriting it. The argument stack is used implicitly and cleared when the call ends.
    Call { decl_id: DeclId, src_dst: RegId },
    /// Append a value onto the end of a string. Uses `to_expanded_string(", ", ...)` on the value.
    /// Used for string interpolation literals. Not the same thing as the `++` operator.
    StringAppend { src_dst: RegId, val: RegId },
    /// Convert a string into a glob. Used for glob interpolation and setting glob variables. If the
    /// value is already a glob, it won't be modified (`no_expand` will have no effect).
    GlobFrom { src_dst: RegId, no_expand: bool },
    /// Push a value onto the end of a list. Used to construct list literals.
    ListPush { src_dst: RegId, item: RegId },
    /// Spread a value onto the end of a list. Used to construct list literals.
    ListSpread { src_dst: RegId, items: RegId },
    /// Insert a key-value pair into a record. Used to construct record literals. Raises an error if
    /// the key already existed in the record.
    RecordInsert {
        src_dst: RegId,
        key: RegId,
        val: RegId,
    },
    /// Spread a record onto a record. Used to construct record literals. Any existing value for the
    /// key is overwritten.
    RecordSpread { src_dst: RegId, items: RegId },
    /// Negate a boolean.
    Not { src_dst: RegId },
    /// Do a binary operation on `lhs_dst` (left) and `rhs` (right) and write the result to
    /// `lhs_dst`.
    BinaryOp {
        lhs_dst: RegId,
        op: Operator,
        rhs: RegId,
    },
    /// Follow a cell path on the value in `src_dst`, storing the result back to `src_dst`
    FollowCellPath { src_dst: RegId, path: RegId },
    /// Clone the value at a cell path in `src`, storing the result to `dst`. The original value
    /// remains in `src`. Must be a collected value.
    CloneCellPath { dst: RegId, src: RegId, path: RegId },
    /// Update/insert a cell path to `new_value` on the value in `src_dst`, storing the modified
    /// value back to `src_dst`
    UpsertCellPath {
        src_dst: RegId,
        path: RegId,
        new_value: RegId,
    },
    /// Jump to an offset in this block
    Jump { index: usize },
    /// Branch to an offset in this block if the value of the `cond` register is a true boolean,
    /// otherwise continue execution
    BranchIf { cond: RegId, index: usize },
    /// Branch to an offset in this block if the value of the `src` register is Empty or Nothing,
    /// otherwise continue execution. The original value in `src` is preserved.
    BranchIfEmpty { src: RegId, index: usize },
    /// Match a pattern on `src`. If the pattern matches, branch to `index` after having set any
    /// variables captured by the pattern. If the pattern doesn't match, continue execution. The
    /// original value is preserved in `src` through this instruction.
    Match {
        pattern: Box<Pattern>,
        src: RegId,
        index: usize,
    },
    /// Check that a match guard is a boolean, throwing
    /// [`MatchGuardNotBool`](crate::ShellError::MatchGuardNotBool) if it isn't. Preserves `src`.
    CheckMatchGuard { src: RegId },
    /// Iterate on register `stream`, putting the next value in `dst` if present, or jumping to
    /// `end_index` if the iterator is finished
    Iterate {
        dst: RegId,
        stream: RegId,
        end_index: usize,
    },
    /// Push an error handler, without capturing the error value
    OnError { index: usize },
    /// Push an error handler, capturing the error value into `dst`. If the error handler is not
    /// called, the register should be freed manually.
    OnErrorInto { index: usize, dst: RegId },
    /// Pop an error handler. This is not necessary when control flow is directed to the error
    /// handler due to an error.
    PopErrorHandler,
    /// Return early from the block, raising a `ShellError::Return` instead.
    ///
    /// Collecting the value is unavoidable.
    ReturnEarly { src: RegId },
    /// Return from the block with the value in the register
    Return { src: RegId },
}

impl Instruction {
    /// Returns a value that can be formatted with [`Display`](std::fmt::Display) to show a detailed
    /// listing of the instruction.
    pub fn display<'a>(
        &'a self,
        engine_state: &'a EngineState,
        data: &'a [u8],
    ) -> FmtInstruction<'a> {
        FmtInstruction {
            engine_state,
            instruction: self,
            data,
        }
    }

    /// Get the output register, for instructions that produce some kind of immediate result.
    pub fn output_register(&self) -> Option<RegId> {
        match *self {
            Instruction::Unreachable => None,
            Instruction::LoadLiteral { dst, .. } => Some(dst),
            Instruction::LoadValue { dst, .. } => Some(dst),
            Instruction::Move { dst, .. } => Some(dst),
            Instruction::Clone { dst, .. } => Some(dst),
            Instruction::Collect { src_dst } => Some(src_dst),
            Instruction::Span { src_dst } => Some(src_dst),
            Instruction::Drop { .. } => None,
            Instruction::Drain { .. } => None,
            Instruction::DrainIfEnd { .. } => None,
            Instruction::LoadVariable { dst, .. } => Some(dst),
            Instruction::StoreVariable { .. } => None,
            Instruction::DropVariable { .. } => None,
            Instruction::LoadEnv { dst, .. } => Some(dst),
            Instruction::LoadEnvOpt { dst, .. } => Some(dst),
            Instruction::StoreEnv { .. } => None,
            Instruction::PushPositional { .. } => None,
            Instruction::AppendRest { .. } => None,
            Instruction::PushFlag { .. } => None,
            Instruction::PushShortFlag { .. } => None,
            Instruction::PushNamed { .. } => None,
            Instruction::PushShortNamed { .. } => None,
            Instruction::PushParserInfo { .. } => None,
            Instruction::RedirectOut { .. } => None,
            Instruction::RedirectErr { .. } => None,
            Instruction::CheckErrRedirected { .. } => None,
            Instruction::OpenFile { .. } => None,
            Instruction::WriteFile { .. } => None,
            Instruction::CloseFile { .. } => None,
            Instruction::Call { src_dst, .. } => Some(src_dst),
            Instruction::StringAppend { src_dst, .. } => Some(src_dst),
            Instruction::GlobFrom { src_dst, .. } => Some(src_dst),
            Instruction::ListPush { src_dst, .. } => Some(src_dst),
            Instruction::ListSpread { src_dst, .. } => Some(src_dst),
            Instruction::RecordInsert { src_dst, .. } => Some(src_dst),
            Instruction::RecordSpread { src_dst, .. } => Some(src_dst),
            Instruction::Not { src_dst } => Some(src_dst),
            Instruction::BinaryOp { lhs_dst, .. } => Some(lhs_dst),
            Instruction::FollowCellPath { src_dst, .. } => Some(src_dst),
            Instruction::CloneCellPath { dst, .. } => Some(dst),
            Instruction::UpsertCellPath { src_dst, .. } => Some(src_dst),
            Instruction::Jump { .. } => None,
            Instruction::BranchIf { .. } => None,
            Instruction::BranchIfEmpty { .. } => None,
            Instruction::Match { .. } => None,
            Instruction::CheckMatchGuard { .. } => None,
            Instruction::Iterate { dst, .. } => Some(dst),
            Instruction::OnError { .. } => None,
            Instruction::OnErrorInto { .. } => None,
            Instruction::PopErrorHandler => None,
            Instruction::ReturnEarly { .. } => None,
            Instruction::Return { .. } => None,
        }
    }

    /// Returns the branch target index of the instruction if this is a branching instruction.
    pub fn branch_target(&self) -> Option<usize> {
        match self {
            Instruction::Jump { index } => Some(*index),
            Instruction::BranchIf { cond: _, index } => Some(*index),
            Instruction::BranchIfEmpty { src: _, index } => Some(*index),
            Instruction::Match {
                pattern: _,
                src: _,
                index,
            } => Some(*index),

            Instruction::Iterate {
                dst: _,
                stream: _,
                end_index,
            } => Some(*end_index),
            Instruction::OnError { index } => Some(*index),
            Instruction::OnErrorInto { index, dst: _ } => Some(*index),
            _ => None,
        }
    }

    /// Sets the branch target of the instruction if this is a branching instruction.
    ///
    /// Returns `Err(target_index)` if it isn't a branching instruction.
    pub fn set_branch_target(&mut self, target_index: usize) -> Result<(), usize> {
        match self {
            Instruction::Jump { index } => *index = target_index,
            Instruction::BranchIf { cond: _, index } => *index = target_index,
            Instruction::BranchIfEmpty { src: _, index } => *index = target_index,
            Instruction::Match {
                pattern: _,
                src: _,
                index,
            } => *index = target_index,

            Instruction::Iterate {
                dst: _,
                stream: _,
                end_index,
            } => *end_index = target_index,
            Instruction::OnError { index } => *index = target_index,
            Instruction::OnErrorInto { index, dst: _ } => *index = target_index,
            _ => return Err(target_index),
        }
        Ok(())
    }

    /// Check for an interrupt before certain instructions
    pub fn check_interrupt(
        &self,
        engine_state: &EngineState,
        span: &Span,
    ) -> Result<(), ShellError> {
        match self {
            Instruction::Jump { .. } | Instruction::Return { .. } => {
                engine_state.signals().check(span)
            }
            _ => Ok(()),
        }
    }
}

// This is to document/enforce the size of `Instruction` in bytes.
// We should try to avoid increasing the size of `Instruction`,
// and PRs that do so will have to change the number below so that it's noted in review.
const _: () = assert!(std::mem::size_of::<Instruction>() <= 24);

/// A literal value that can be embedded in an instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    Filesize(Filesize),
    Duration(i64),
    Binary(DataSlice),
    Block(BlockId),
    Closure(BlockId),
    RowCondition(BlockId),
    Range {
        start: RegId,
        step: RegId,
        end: RegId,
        inclusion: RangeInclusion,
    },
    List {
        capacity: usize,
    },
    Record {
        capacity: usize,
    },
    Filepath {
        val: DataSlice,
        no_expand: bool,
    },
    Directory {
        val: DataSlice,
        no_expand: bool,
    },
    GlobPattern {
        val: DataSlice,
        no_expand: bool,
    },
    String(DataSlice),
    RawString(DataSlice),
    CellPath(Box<CellPath>),
    Date(Box<DateTime<FixedOffset>>),
    Nothing,
}

/// A redirection mode for the next call. See [`OutDest`](crate::OutDest).
///
/// This is generated by:
///
/// 1. Explicit redirection in a [`PipelineElement`](crate::ast::PipelineElement), or
/// 2. The [`pipe_redirection()`](crate::engine::Command::pipe_redirection) of the command being
///    piped into.
///
/// Not setting it uses the default, determined by [`Stack`](crate::engine::Stack).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RedirectMode {
    Pipe,
    PipeSeparate,
    Value,
    Null,
    Inherit,
    Print,
    /// Use the given numbered file.
    File {
        file_num: u32,
    },
    /// Use the redirection mode requested by the caller, for a pre-return call.
    Caller,
}

/// Just a hack to allow `Arc<[u8]>` to be serialized and deserialized
mod serde_arc_u8_array {
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    pub fn serialize<S>(data: &Arc<[u8]>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        data.as_ref().serialize(ser)
    }

    pub fn deserialize<'de, D>(de: D) -> Result<Arc<[u8]>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data: Vec<u8> = Deserialize::deserialize(de)?;
        Ok(data.into())
    }
}
