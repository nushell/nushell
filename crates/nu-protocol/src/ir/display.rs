use std::fmt;

use crate::{engine::EngineState, DeclId};

use super::{Instruction, IrBlock, RedirectMode};

pub struct FmtIrBlock<'a> {
    pub(super) engine_state: &'a EngineState,
    pub(super) ir_block: &'a IrBlock,
}

impl<'a> fmt::Display for FmtIrBlock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = |count| if count == 1 { "" } else { "s" };
        writeln!(
            f,
            "# {} register{}, {} instruction{}",
            self.ir_block.register_count,
            plural(self.ir_block.register_count),
            self.ir_block.instructions.len(),
            plural(self.ir_block.instructions.len()),
        )?;
        for (index, instruction) in self.ir_block.instructions.iter().enumerate() {
            writeln!(
                f,
                "{:-4}: {}",
                index,
                FmtInstruction {
                    engine_state: self.engine_state,
                    instruction
                }
            )?;
        }
        Ok(())
    }
}

struct FmtInstruction<'a> {
    engine_state: &'a EngineState,
    instruction: &'a Instruction,
}

impl<'a> fmt::Display for FmtInstruction<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const WIDTH: usize = 20;

        match self.instruction {
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
                let decl = FmtDecl::new(self.engine_state, *decl_id);
                write!(f, "{:WIDTH$} {decl}, {src_dst}", "call")
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

struct FmtDecl<'a>(DeclId, &'a str);

impl<'a> FmtDecl<'a> {
    fn new(engine_state: &'a EngineState, decl_id: DeclId) -> Self {
        FmtDecl(decl_id, engine_state.get_decl(decl_id).name())
    }
}

impl fmt::Display for FmtDecl<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "decl {} {:?}", self.0, self.1)
    }
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
