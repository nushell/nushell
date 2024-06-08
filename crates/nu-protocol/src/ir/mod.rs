use std::fmt;

use crate::{
    ast::{CellPath, Operator},
    BlockId, DeclId, Range, RegId, Span,
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
    /// Append a list in `src` to the list in `dst`, modifying `dst`. Used for list spread
    AppendList { dst: RegId, src: RegId },
    /// Append a string in `src` to the string in `dst`, modifying `dst`. Used for string
    /// interpolation
    AppendString { dst: RegId, src: RegId },
    /// Copy a register (must be a collected value)
    Clone { dst: RegId, src: RegId },
    /// Collect a stream in a register to a value
    Collect { src_dst: RegId },
    /// Add a positional arg to the next call
    PushPositional { src: RegId },
    /// Add a list of args to the next call (spread/rest)
    AppendRest { src: RegId },
    /// Add a named arg to the next call. The `src` is optional, can be `None` if there is no value.
    PushNamed { name: Box<str>, src: Option<RegId> },
    /// Clear the argument stack for the next call
    ClearArgs,
    /// Make a call. The input is taken from `src_dst`, and the output is placed in `src_dst`,
    /// overwriting it. The argument stack is used implicitly and cleared when the call ends.
    Call { decl_id: DeclId, src_dst: RegId },
    /// Do a binary operation on `lhs` (left) and `rhs` (right) and write the result to `dst`.
    BinaryOp {
        dst: RegId,
        op: Operator,
        lhs: RegId,
        rhs: RegId,
    },
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
        match self {
            Instruction::LoadLiteral { dst, lit } => {
                write!(f, "{:15} %{}, {:?}", "load-literal", dst.0, lit)
            }
            Instruction::AppendList { dst, src } => {
                write!(f, "{:15} %{}, %{}", "append-list", dst.0, src.0)
            }
            Instruction::AppendString { dst, src } => {
                write!(f, "{:15} %{}, %{}", "append-string", dst.0, src.0)
            }
            Instruction::Clone { dst, src } => {
                write!(f, "{:15} %{}, %{}", "clone", dst.0, src.0)
            }
            Instruction::Collect { src_dst } => {
                write!(f, "{:15} %{}", "clone", src_dst.0)
            }
            Instruction::PushPositional { src } => {
                write!(f, "{:15} %{}", "push-positional", src.0)
            }
            Instruction::AppendRest { src } => {
                write!(f, "{:15} %{}", "append-rest", src.0)
            }
            Instruction::PushNamed { name, src } => {
                if let Some(src) = src {
                    write!(f, "{:15} {:?}, %{}", "push-named", name, src.0)
                } else {
                    write!(f, "{:15} {:?}", "push-named", name)
                }
            }
            Instruction::ClearArgs => {
                write!(f, "{:15}", "clear-args")
            }
            Instruction::Call { decl_id, src_dst } => {
                write!(f, "{:15} decl {}, %{}", "call", decl_id, src_dst.0)
            }
            Instruction::BinaryOp { dst, lhs, op, rhs } => {
                write!(
                    f,
                    "{:15} %{}, {:?}, %{}, %{}",
                    "binary-op", dst.0, op, lhs.0, rhs.0
                )
            }
            Instruction::Jump { index } => {
                write!(f, "{:15} {}", "jump", index)
            }
            Instruction::BranchIf { cond, index } => {
                write!(f, "{:15} %{}, {}", "branch-if", cond.0, index)
            }
            Instruction::Return { src } => {
                write!(f, "{:15} %{}", "return", src.0)
            }
        }
    }
}

// This is to document/enforce the size of `Instruction` in bytes.
// We should try to avoid increasing the size of `Instruction`,
// and PRs that do so will have to change the number below so that it's noted in review.
const _: () = assert!(std::mem::size_of::<Instruction>() <= 40);

#[derive(Debug, Clone)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    Binary(Vec<u8>),
    Range(Box<Range>),
    Block(BlockId),
    Closure(BlockId),
    List(Vec<Literal>),
    Filepath(String, bool),
    Directory(String, bool),
    GlobPattern(String, bool),
    String(String),
    RawString(String),
    CellPath(CellPath),
    Nothing,
}

#[test]
fn dummy_test() {
    use crate::ast::Math;

    let ir_block = IrBlock {
        instructions: vec![
            Instruction::LoadLiteral {
                dst: RegId(0),
                lit: Literal::String("foo".into()),
            },
            Instruction::PushPositional { src: RegId(0) },
            Instruction::LoadLiteral {
                dst: RegId(2),
                lit: Literal::Int(40),
            },
            Instruction::LoadLiteral {
                dst: RegId(3),
                lit: Literal::Int(25),
            },
            Instruction::BinaryOp {
                dst: RegId(1),
                op: Operator::Math(Math::Plus),
                lhs: RegId(2),
                rhs: RegId(3),
            },
            Instruction::PushNamed {
                name: "bar-level".into(),
                src: Some(RegId(1)),
            },
            Instruction::LoadLiteral {
                dst: RegId(4),
                lit: Literal::Nothing,
            },
            Instruction::Call {
                decl_id: 40,
                src_dst: RegId(4),
            },
            Instruction::Return { src: RegId(4) },
        ],
        spans: vec![],
        register_count: 5,
    };
    println!("{}", ir_block);
    todo!();
}
