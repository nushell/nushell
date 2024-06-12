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
    pub instructions: Vec<Instruction>,
    pub spans: Vec<Span>,
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
    /// Add a positional arg to the next call
    PushPositional { src: RegId },
    /// Add a list of args to the next call (spread/rest)
    AppendRest { src: RegId },
    /// Add a named arg with no value to the next call.
    PushFlag { name: Box<str> },
    /// Add a named arg with a value to the next call.
    PushNamed { name: Box<str>, src: RegId },
    /// Set the redirection for stdout for the next call (only)
    RedirectOut { mode: RedirectMode },
    /// Set the redirection for stderr for the next call (only)
    RedirectErr { mode: RedirectMode },
    /// Make a call. The input is taken from `src_dst`, and the output is placed in `src_dst`,
    /// overwriting it. The argument stack is used implicitly and cleared when the call ends.
    Call { decl_id: DeclId, src_dst: RegId },
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
    pub fn display<'a>(&'a self, engine_state: &'a EngineState) -> FmtInstruction<'a> {
        FmtInstruction {
            engine_state,
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
