use std::sync::Arc;

use crate::{
    ast::{CellPath, Operator, RangeInclusion},
    engine::EngineState,
    BlockId, DeclId, RegId, Span, VarId,
};

use serde::{Deserialize, Serialize};

mod call;
mod display;

pub use call::*;
pub use display::{FmtInstruction, FmtIrBlock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrBlock {
    pub instructions: Vec<Instruction>,
    pub spans: Vec<Span>,
    #[serde(with = "serde_arc_u8_array")]
    pub data: Arc<[u8]>,
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

/// A slice into the `data` array of a block. This is a compact and cache-friendly way to store
/// string data that a block uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DataSlice {
    pub start: u32,
    pub len: u32,
}

impl std::ops::Index<DataSlice> for [u8] {
    type Output = [u8];

    fn index(&self, index: DataSlice) -> &Self::Output {
        &self[index.start as usize..(index.start as usize + index.len as usize)]
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
    /// Drop the value/straem in a register, without draining
    Drop { src: RegId },
    /// Drain the value/stream in a register and discard (e.g. semicolon)
    Drain { src: RegId },
    /// Load the value of a variable into the `dst` register
    LoadVariable { dst: RegId, var_id: VarId },
    /// Store the value of a variable from the `src` register
    StoreVariable { var_id: VarId, src: RegId },
    /// Load the value of an environment variable into the `dst` register
    LoadEnv { dst: RegId, key: DataSlice },
    /// Load the value of an environment variable into the `dst` register, or `Nothing` if it
    /// doesn't exist
    LoadEnvOpt { dst: RegId, key: DataSlice },
    /// Store the value of an environment variable from the `src` register
    StoreEnv { key: DataSlice, src: RegId },
    /// Add a positional arg to the next call
    PushPositional { src: RegId },
    /// Add a list of args to the next call (spread/rest)
    AppendRest { src: RegId },
    /// Add a named arg with no value to the next call.
    PushFlag { name: DataSlice },
    /// Add a named arg with a value to the next call.
    PushNamed { name: DataSlice, src: RegId },
    /// Set the redirection for stdout for the next call (only)
    RedirectOut { mode: RedirectMode },
    /// Set the redirection for stderr for the next call (only)
    RedirectErr { mode: RedirectMode },
    /// Make a call. The input is taken from `src_dst`, and the output is placed in `src_dst`,
    /// overwriting it. The argument stack is used implicitly and cleared when the call ends.
    Call { decl_id: DeclId, src_dst: RegId },
    /// Push a value onto the end of a list. Used to construct list literals.
    ListPush { src_dst: RegId, item: RegId },
    /// Spread a value onto the end of a list. Used to construct list literals.
    ListSpread { src_dst: RegId, items: RegId },
    /// Insert a key-value pair into a record. Used to construct record literals. Any existing value
    /// for the key is overwritten.
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
    Binary(DataSlice),
    Block(BlockId),
    Closure(BlockId),
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
