use std::fmt;

use crate::{
    ast::{CellPath, Operator},
    BlockId, DeclId, RegId, Span,
};

#[derive(Debug, Clone)]
pub struct IrBlock {
    pub instructions: Vec<Instruction>,
    pub spans: Vec<Span>,
    pub register_count: usize,
}

impl fmt::Display for IrBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "# {} registers, {} instructions",
            self.register_count,
            self.instructions.len()
        )?;
        for instruction in &self.instructions {
            writeln!(f, "{}", instruction)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
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
    /// Follow a cell path on the `path`
    FollowCellPath { src_dst: RegId, path: RegId },
    /// Jump to an offset in this block
    Jump { index: usize },
    /// Branch to an offset in this block if the value of the `cond` register is a true boolean,
    /// otherwise continue execution
    BranchIf { cond: RegId, index: usize },
    /// Return from the block with the value in the register
    Return { src: RegId },
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const WIDTH: usize = 20;

        match self {
            Instruction::LoadLiteral { dst, lit } => {
                write!(f, "{:WIDTH$} {dst}, {lit:?}", "load-literal")
            }
            Instruction::Move { dst, src } => {
                write!(f, "{:WIDTH$} {dst}, {src}", "move")
            }
            Instruction::Clone { dst, src } => {
                write!(f, "{:WIDTH$} {dst}, {src}", "clone")
            }
            Instruction::Collect { src_dst } => {
                write!(f, "{:WIDTH$} {src_dst}", "collect")
            }
            Instruction::Drain { src } => {
                write!(f, "{:WIDTH$} {src}", "drain")
            }
            Instruction::PushPositional { src } => {
                write!(f, "{:WIDTH$} {src}", "push-positional")
            }
            Instruction::AppendRest { src } => {
                write!(f, "{:WIDTH$} {src}", "append-rest")
            }
            Instruction::PushFlag { name } => {
                write!(f, "{:WIDTH$} {name:?}", "push-flag")
            }
            Instruction::PushNamed { name, src } => {
                write!(f, "{:WIDTH$} {name:?}, {src}", "push-named")
            }
            Instruction::RedirectOut { mode } => {
                write!(f, "{:WIDTH$} {mode}", "redirect-out")
            }
            Instruction::RedirectErr { mode } => {
                write!(f, "{:WIDTH$} {mode}", "redirect-err")
            }
            Instruction::Call { decl_id, src_dst } => {
                write!(f, "{:WIDTH$} decl {decl_id}, {src_dst}", "call")
            }
            Instruction::BinaryOp { lhs_dst, op, rhs } => {
                write!(f, "{:WIDTH$} {lhs_dst}, {op:?}, {rhs}", "binary-op")
            }
            Instruction::FollowCellPath { src_dst, path } => {
                write!(f, "{:WIDTH$} {src_dst}, {path}", "follow-cell-path")
            }
            Instruction::Jump { index } => {
                write!(f, "{:WIDTH$} {index}", "jump")
            }
            Instruction::BranchIf { cond, index } => {
                write!(f, "{:WIDTH$} {cond}, {index}", "branch-if")
            }
            Instruction::Return { src } => {
                write!(f, "{:WIDTH$} {src}", "return")
            }
        }
    }
}

// This is to document/enforce the size of `Instruction` in bytes.
// We should try to avoid increasing the size of `Instruction`,
// and PRs that do so will have to change the number below so that it's noted in review.
const _: () = assert!(std::mem::size_of::<Instruction>() <= 32);

/// A literal value that can be embedded in an instruction.
#[derive(Debug, Clone)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    Binary(Box<[u8]>),
    Block(BlockId),
    Closure(BlockId),
    List(Box<[Literal]>),
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
#[derive(Debug, Clone, Copy)]
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

impl std::fmt::Display for RedirectMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedirectMode::Pipe => write!(f, "pipe"),
            RedirectMode::Capture => write!(f, "capture"),
            RedirectMode::Null => write!(f, "null"),
            RedirectMode::Inherit => write!(f, "inherit"),
            RedirectMode::File { path, append } => write!(f, "file({path}, append={append})"),
        }
    }
}

#[test]
fn dummy_test() {
    use crate::ast::Math;

    let ir_block = IrBlock {
        instructions: vec![
            Instruction::LoadLiteral {
                dst: RegId(1),
                lit: Literal::String("foo".into()),
            },
            Instruction::PushPositional { src: RegId(1) },
            Instruction::LoadLiteral {
                dst: RegId(1),
                lit: Literal::Int(40),
            },
            Instruction::LoadLiteral {
                dst: RegId(2),
                lit: Literal::Int(25),
            },
            Instruction::BinaryOp {
                lhs_dst: RegId(1),
                op: Operator::Math(Math::Plus),
                rhs: RegId(2),
            },
            Instruction::PushNamed {
                name: "bar-level".into(),
                src: RegId(1),
            },
            Instruction::LoadLiteral {
                dst: RegId(1),
                lit: Literal::Nothing,
            },
            Instruction::Call {
                decl_id: 40,
                src_dst: RegId(1),
            },
            Instruction::Return { src: RegId(1) },
        ],
        spans: vec![],
        register_count: 2,
    };
    println!("{}", ir_block);
    todo!();
}
