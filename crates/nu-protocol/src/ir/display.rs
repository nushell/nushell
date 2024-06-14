use std::fmt::{self, Write};

use crate::{engine::EngineState, DeclId, VarId};

use super::{CallArg, Instruction, IrBlock, Literal, RedirectMode};

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
                    call_args: &self.ir_block.call_args,
                    instruction
                }
            )?;
        }
        Ok(())
    }
}

pub struct FmtInstruction<'a> {
    pub(super) engine_state: &'a EngineState,
    pub(super) call_args: &'a [CallArg],
    pub(super) instruction: &'a Instruction,
}

impl<'a> fmt::Display for FmtInstruction<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const WIDTH: usize = 20;

        match self.instruction {
            Instruction::LoadLiteral { dst, lit } => {
                write!(f, "{:WIDTH$} {dst}, {lit}", "load-literal")
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
            Instruction::LoadVariable { dst, var_id } => {
                let var = FmtVar::new(self.engine_state, *var_id);
                write!(f, "{:WIDTH$} {dst}, {var}", "load-variable")
            }
            Instruction::StoreVariable { var_id, src } => {
                let var = FmtVar::new(self.engine_state, *var_id);
                write!(f, "{:WIDTH$} {var}, {src}", "store-variable")
            }
            Instruction::LoadEnv { dst, key } => {
                write!(f, "{:WIDTH$} {dst}, {key:?}", "load-env")
            }
            Instruction::LoadEnvOpt { dst, key } => {
                write!(f, "{:WIDTH$} {dst}, {key:?}", "load-env-opt")
            }
            Instruction::StoreEnv { key, src } => {
                write!(f, "{:WIDTH$} {key:?}, {src}", "store-env")
            }
            Instruction::RedirectOut { mode } => {
                write!(f, "{:WIDTH$} {mode}", "redirect-out")
            }
            Instruction::RedirectErr { mode } => {
                write!(f, "{:WIDTH$} {mode}", "redirect-err")
            }
            Instruction::Call {
                decl_id,
                src_dst,
                args_start,
                args_len,
            } => {
                let decl = FmtDecl::new(self.engine_state, *decl_id);
                let args = FmtCallArgs {
                    call_args: self.call_args,
                    args_start: *args_start,
                    args_len: *args_len,
                };
                write!(f, "{:WIDTH$} {decl}, {src_dst}, {args}", "call")
            }
            Instruction::BinaryOp { lhs_dst, op, rhs } => {
                write!(f, "{:WIDTH$} {lhs_dst}, {op:?}, {rhs}", "binary-op")
            }
            Instruction::FollowCellPath { src_dst, path } => {
                write!(f, "{:WIDTH$} {src_dst}, {path}", "follow-cell-path")
            }
            Instruction::CloneCellPath { dst, src, path } => {
                write!(f, "{:WIDTH$} {dst}, {src}, {path}", "clone-cell-path")
            }
            Instruction::UpsertCellPath {
                src_dst,
                path,
                new_value,
            } => {
                write!(
                    f,
                    "{:WIDTH$} {src_dst}, {path}, {new_value}",
                    "upsert-cell-path"
                )
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

struct FmtCallArgs<'a> {
    call_args: &'a [CallArg],
    args_start: usize,
    args_len: usize,
}

impl fmt::Display for FmtCallArgs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('[')?;
        for index in 0..self.args_len {
            if index != 0 {
                f.write_str(", ")?;
            }
            if let Some(arg) = self.call_args.get(self.args_start + index) {
                match arg {
                    CallArg::Positional(reg) => write!(f, "{reg}")?,
                    CallArg::Spread(reg) => write!(f, "...{reg}")?,
                    CallArg::Flag(name) => write!(f, "--{name}")?,
                    CallArg::Named(name, reg) => write!(f, "--{name} {reg}")?,
                }
            } else {
                f.write_str("<missing>")?;
            }
        }
        f.write_char(']')
    }
}

struct FmtVar<'a>(DeclId, Option<&'a str>);

impl<'a> FmtVar<'a> {
    fn new(engine_state: &'a EngineState, var_id: VarId) -> Self {
        // Search for the name of the variable
        let name: Option<&str> = engine_state
            .active_overlays(&[])
            .flat_map(|overlay| overlay.vars.iter())
            .find(|(_, v)| **v == var_id)
            .map(|(k, _)| std::str::from_utf8(k).unwrap_or("<utf-8 error>"));
        FmtVar(var_id, name)
    }
}

impl fmt::Display for FmtVar<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.1 {
            write!(f, "var {} {:?}", self.0, name)
        } else {
            write!(f, "var {}", self.0)
        }
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

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::CellPath(cell_path) => write!(f, "CellPath({})", cell_path),
            _ => write!(f, "{:?}", self),
        }
    }
}
