use crate::{
    ast::{CellPath, Operator},
    engine::EngineState,
    BlockId, DeclId, RegId, Span, Spanned, VarId,
};

use serde::{Deserialize, Serialize};

mod display;
pub use display::{FmtInstruction, FmtIrBlock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrBlock {
    /// Instructions to execute in sequence in the block.
    ///
    /// Execution starts at index zero. A [`Return`](Instruction::Return) instruction must be
    /// present.
    pub instructions: Vec<Instruction>,
    /// Spans for each instruction. Indexes are matched 1:1 with the instructions, so this can be
    /// zipped if desired.
    pub spans: Vec<Span>,
    /// Array that describes arguments for [`Call`](Instruction::Call) instructions, sliced into by
    /// the `args_start` and `args_len` fields.
    pub call_args: Vec<CallArg>,
    /// The number of registers to allocate at runtime. This number is statically determined during
    /// compilation, and can't be adjusted dynamically.
    pub register_count: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    /// Load a literal value into the `dst` register
    LoadLiteral { dst: RegId, lit: Literal },
    /// Move a register. Value is taken from `src` (used by this instruction).
    Move { dst: RegId, src: RegId },
    /// Copy a register (must be a collected value). Value is still in `src` after this instruction.
    Clone { dst: RegId, src: RegId },
    /// Collect a stream in a register to a value
    Collect { src_dst: RegId },
    /// Drain the value/stream in a register and discard (e.g. semicolon)
    Drain { src: RegId },
    /// Load the value of a variable into the `dst` register
    LoadVariable { dst: RegId, var_id: VarId },
    /// Store the value of a variable from the `src` register
    StoreVariable { var_id: VarId, src: RegId },
    /// Load the value of an environment variable into the `dst` register
    LoadEnv { dst: RegId, key: Box<str> },
    /// Load the value of an environment variable into the `dst` register, or `Nothing` if it
    /// doesn't exist
    LoadEnvOpt { dst: RegId, key: Box<str> },
    /// Store the value of an environment variable from the `src` register
    StoreEnv { key: Box<str>, src: RegId },
    /// Set the redirection for stdout for the next call (only)
    RedirectOut { mode: RedirectMode },
    /// Set the redirection for stderr for the next call (only)
    RedirectErr { mode: RedirectMode },
    /// Make a call. The input is taken from `src_dst`, and the output is placed in `src_dst`,
    /// overwriting it.
    ///
    /// Call arguments are specified by the `args_start` and `args_len` fields, which point at a
    /// range of values within the `arguments` array, and both may be zero for a zero-arg call.
    Call {
        decl_id: DeclId,
        src_dst: RegId,
        args_start: usize,
        args_len: usize,
    },
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
    /// Update a cell path
    /// Jump to an offset in this block
    Jump { index: usize },
    /// Branch to an offset in this block if the value of the `cond` register is a true boolean,
    /// otherwise continue execution
    BranchIf { cond: RegId, index: usize },
    /// Return from the block with the value in the register
    Return { src: RegId },
}

impl Instruction {
    /// Returns a value that can be formatted with [`Display`](std::fmt::Display) to show a detailed
    /// listing of the instruction.
    pub fn display<'a>(
        &'a self,
        engine_state: &'a EngineState,
        call_args: &'a [CallArg],
    ) -> FmtInstruction<'a> {
        FmtInstruction {
            engine_state,
            call_args,
            instruction: self,
        }
    }
}

// This is to document/enforce the size of `Instruction` in bytes.
// We should try to avoid increasing the size of `Instruction`,
// and PRs that do so will have to change the number below so that it's noted in review.
const _: () = assert!(std::mem::size_of::<Instruction>() <= 32);

/// A literal value that can be embedded in an instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    Binary(Box<[u8]>),
    Block(BlockId),
    Closure(BlockId),
    List(Box<[Spanned<Literal>]>),
    Filepath { val: Box<str>, no_expand: bool },
    Directory { val: Box<str>, no_expand: bool },
    GlobPattern { val: Box<str>, no_expand: bool },
    String(Box<str>),
    RawString(Box<str>),
    CellPath(Box<CellPath>),
    Nothing,
}

/// Describes an argument made to a call. This enum is contained within the `arguments` array of an
/// [`IrBlock`], which is referenced into by [`Instruction::Call`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallArg {
    Positional(RegId),
    Spread(RegId),
    /// Like `Named`, but with no value. Smaller than using an `Option`.
    Flag(Box<str>),
    Named(Box<str>, RegId),
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
    Capture,
    Null,
    Inherit,
    /// File path to be used in register.
    File {
        path: RegId,
        append: bool,
    },
}
