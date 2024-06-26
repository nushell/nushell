use std::fmt;

use crate::{engine::EngineState, DeclId, VarId};

use super::{DataSlice, Instruction, IrBlock, Literal, RedirectMode};

pub struct FmtIrBlock<'a> {
    pub(super) engine_state: &'a EngineState,
    pub(super) ir_block: &'a IrBlock,
}

impl<'a> fmt::Display for FmtIrBlock<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = |count| if count == 1 { "" } else { "s" };
        writeln!(
            f,
            "# {} register{}, {} instruction{}, {} byte{} of data",
            self.ir_block.register_count,
            plural(self.ir_block.register_count),
            self.ir_block.instructions.len(),
            plural(self.ir_block.instructions.len()),
            self.ir_block.data.len(),
            plural(self.ir_block.data.len()),
        )?;
        for (index, instruction) in self.ir_block.instructions.iter().enumerate() {
            writeln!(
                f,
                "{:-4}: {}",
                index,
                FmtInstruction {
                    engine_state: self.engine_state,
                    instruction,
                    data: &self.ir_block.data,
                }
            )?;
        }
        Ok(())
    }
}

pub struct FmtInstruction<'a> {
    pub(super) engine_state: &'a EngineState,
    pub(super) instruction: &'a Instruction,
    pub(super) data: &'a [u8],
}

impl<'a> fmt::Display for FmtInstruction<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const WIDTH: usize = 20;

        match self.instruction {
            Instruction::LoadLiteral { dst, lit } => {
                let lit = FmtLiteral {
                    literal: lit,
                    data: self.data,
                };
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
                let key = FmtData(self.data, *key);
                write!(f, "{:WIDTH$} {dst}, {key}", "load-env")
            }
            Instruction::LoadEnvOpt { dst, key } => {
                let key = FmtData(self.data, *key);
                write!(f, "{:WIDTH$} {dst}, {key}", "load-env-opt")
            }
            Instruction::StoreEnv { key, src } => {
                let key = FmtData(self.data, *key);
                write!(f, "{:WIDTH$} {key}, {src}", "store-env")
            }
            Instruction::PushPositional { src } => {
                write!(f, "{:WIDTH$} {src}", "push-positional")
            }
            Instruction::AppendRest { src } => {
                write!(f, "{:WIDTH$} {src}", "append-rest")
            }
            Instruction::PushFlag { name } => {
                let name = FmtData(self.data, *name);
                write!(f, "{:WIDTH$} {name}", "push-flag")
            }
            Instruction::PushNamed { name, src } => {
                let name = FmtData(self.data, *name);
                write!(f, "{:WIDTH$} {name}, {src}", "push-named")
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
            Instruction::ListPush { src_dst, item } => {
                write!(f, "{:WIDTH$} {src_dst}, {item}", "list-push")
            }
            Instruction::RecordInsert { src_dst, key, val } => {
                write!(f, "{:WIDTH$} {src_dst}, {key}, {val}", "record-insert")
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

impl fmt::Display for RedirectMode {
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

struct FmtData<'a>(&'a [u8], DataSlice);

impl<'a> fmt::Display for FmtData<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(s) = std::str::from_utf8(&self.0[self.1]) {
            // Write as string
            write!(f, "{s:?}")
        } else {
            // Write as byte array
            write!(f, "0x{:x?}", self.0)
        }
    }
}

struct FmtLiteral<'a> {
    literal: &'a Literal,
    data: &'a [u8],
}

impl<'a> fmt::Display for FmtLiteral<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.literal {
            Literal::Bool(b) => write!(f, "bool({b:?})"),
            Literal::Int(i) => write!(f, "int({i:?})"),
            Literal::Float(fl) => write!(f, "float({fl:?})"),
            Literal::Binary(b) => write!(f, "binary({})", FmtData(self.data, *b)),
            Literal::Block(id) => write!(f, "block({id})"),
            Literal::Closure(id) => write!(f, "closure({id})"),
            Literal::Range {
                start,
                step,
                end,
                inclusion,
            } => write!(f, "range({start}, {step}, {end}, {inclusion:?})"),
            Literal::List { capacity } => write!(f, "list(capacity = {capacity})"),
            Literal::Record { capacity } => write!(f, "record(capacity = {capacity})"),
            Literal::Filepath { val, no_expand } => write!(
                f,
                "filepath({}, no_expand = {no_expand:?})",
                FmtData(self.data, *val)
            ),
            Literal::Directory { val, no_expand } => write!(
                f,
                "directory({}, no_expand = {no_expand:?})",
                FmtData(self.data, *val)
            ),
            Literal::GlobPattern { val, no_expand } => write!(
                f,
                "glob-pattern({}, no_expand = {no_expand:?})",
                FmtData(self.data, *val)
            ),
            Literal::String(s) => write!(f, "string({})", FmtData(self.data, *s)),
            Literal::RawString(rs) => write!(f, "raw-string({})", FmtData(self.data, *rs)),
            Literal::CellPath(p) => write!(f, "cell-path({p})"),
            Literal::Nothing => write!(f, "nothing"),
        }
    }
}
