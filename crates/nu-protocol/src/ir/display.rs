use super::{DataSlice, Instruction, IrBlock, Literal, RedirectMode};
use crate::{DeclId, VarId, ast::Pattern, engine::EngineState};
use std::fmt::{self};

pub struct FmtIrBlock<'a> {
    pub(super) engine_state: &'a EngineState,
    pub(super) ir_block: &'a IrBlock,
}

impl fmt::Display for FmtIrBlock<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plural = |count| if count == 1 { "" } else { "s" };
        writeln!(
            f,
            "# {} register{}, {} instruction{}, {} byte{} of data",
            self.ir_block.register_count,
            plural(self.ir_block.register_count as usize),
            self.ir_block.instructions.len(),
            plural(self.ir_block.instructions.len()),
            self.ir_block.data.len(),
            plural(self.ir_block.data.len()),
        )?;
        if self.ir_block.file_count > 0 {
            writeln!(
                f,
                "# {} file{} used for redirection",
                self.ir_block.file_count,
                plural(self.ir_block.file_count as usize)
            )?;
        }
        for (index, instruction) in self.ir_block.instructions.iter().enumerate() {
            let formatted = format!(
                "{:-4}: {}",
                index,
                FmtInstruction {
                    engine_state: self.engine_state,
                    instruction,
                    data: &self.ir_block.data,
                }
            );
            let comment = &self.ir_block.comments[index];
            if comment.is_empty() {
                writeln!(f, "{formatted}")?;
            } else {
                writeln!(f, "{formatted:40} # {comment}")?;
            }
        }
        Ok(())
    }
}

pub struct FmtInstruction<'a> {
    pub(super) engine_state: &'a EngineState,
    pub(super) instruction: &'a Instruction,
    pub(super) data: &'a [u8],
}

impl fmt::Display for FmtInstruction<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const WIDTH: usize = 22;

        match self.instruction {
            Instruction::Unreachable => {
                write!(f, "{:WIDTH$}", "unreachable")
            }
            Instruction::LoadLiteral { dst, lit } => {
                let lit = FmtLiteral {
                    literal: lit,
                    data: self.data,
                };
                write!(f, "{:WIDTH$} {dst}, {lit}", "load-literal")
            }
            Instruction::LoadValue { dst, val } => {
                let val = val.to_debug_string();
                write!(f, "{:WIDTH$} {dst}, {val}", "load-value")
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
            Instruction::Span { src_dst } => {
                write!(f, "{:WIDTH$} {src_dst}", "span")
            }
            Instruction::Drop { src } => {
                write!(f, "{:WIDTH$} {src}", "drop")
            }
            Instruction::Drain { src } => {
                write!(f, "{:WIDTH$} {src}", "drain")
            }
            Instruction::DrainIfEnd { src } => {
                write!(f, "{:WIDTH$} {src}", "drain-if-end")
            }
            Instruction::LoadVariable { dst, var_id } => {
                let var = FmtVar::new(self.engine_state, *var_id);
                write!(f, "{:WIDTH$} {dst}, {var}", "load-variable")
            }
            Instruction::StoreVariable { var_id, src } => {
                let var = FmtVar::new(self.engine_state, *var_id);
                write!(f, "{:WIDTH$} {var}, {src}", "store-variable")
            }
            Instruction::DropVariable { var_id } => {
                let var = FmtVar::new(self.engine_state, *var_id);
                write!(f, "{:WIDTH$} {var}", "drop-variable")
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
            Instruction::PushShortFlag { short } => {
                let short = FmtData(self.data, *short);
                write!(f, "{:WIDTH$} {short}", "push-short-flag")
            }
            Instruction::PushNamed { name, src } => {
                let name = FmtData(self.data, *name);
                write!(f, "{:WIDTH$} {name}, {src}", "push-named")
            }
            Instruction::PushShortNamed { short, src } => {
                let short = FmtData(self.data, *short);
                write!(f, "{:WIDTH$} {short}, {src}", "push-short-named")
            }
            Instruction::PushParserInfo { name, info } => {
                let name = FmtData(self.data, *name);
                write!(f, "{:WIDTH$} {name}, {info:?}", "push-parser-info")
            }
            Instruction::RedirectOut { mode } => {
                write!(f, "{:WIDTH$} {mode}", "redirect-out")
            }
            Instruction::RedirectErr { mode } => {
                write!(f, "{:WIDTH$} {mode}", "redirect-err")
            }
            Instruction::CheckErrRedirected { src } => {
                write!(f, "{:WIDTH$} {src}", "check-err-redirected")
            }
            Instruction::OpenFile {
                file_num,
                path,
                append,
            } => {
                write!(
                    f,
                    "{:WIDTH$} file({file_num}), {path}, append = {append:?}",
                    "open-file"
                )
            }
            Instruction::WriteFile { file_num, src } => {
                write!(f, "{:WIDTH$} file({file_num}), {src}", "write-file")
            }
            Instruction::CloseFile { file_num } => {
                write!(f, "{:WIDTH$} file({file_num})", "close-file")
            }
            Instruction::Call { decl_id, src_dst } => {
                let decl = FmtDecl::new(self.engine_state, *decl_id);
                write!(f, "{:WIDTH$} {decl}, {src_dst}", "call")
            }
            Instruction::StringAppend { src_dst, val } => {
                write!(f, "{:WIDTH$} {src_dst}, {val}", "string-append")
            }
            Instruction::GlobFrom { src_dst, no_expand } => {
                let no_expand = if *no_expand { "no-expand" } else { "expand" };
                write!(f, "{:WIDTH$} {src_dst}, {no_expand}", "glob-from",)
            }
            Instruction::ListPush { src_dst, item } => {
                write!(f, "{:WIDTH$} {src_dst}, {item}", "list-push")
            }
            Instruction::ListSpread { src_dst, items } => {
                write!(f, "{:WIDTH$} {src_dst}, {items}", "list-spread")
            }
            Instruction::RecordInsert { src_dst, key, val } => {
                write!(f, "{:WIDTH$} {src_dst}, {key}, {val}", "record-insert")
            }
            Instruction::RecordSpread { src_dst, items } => {
                write!(f, "{:WIDTH$} {src_dst}, {items}", "record-spread")
            }
            Instruction::Not { src_dst } => {
                write!(f, "{:WIDTH$} {src_dst}", "not")
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
            Instruction::BranchIfEmpty { src, index } => {
                write!(f, "{:WIDTH$} {src}, {index}", "branch-if-empty")
            }
            Instruction::Match {
                pattern,
                src,
                index,
            } => {
                let pattern = FmtPattern {
                    engine_state: self.engine_state,
                    pattern,
                };
                write!(f, "{:WIDTH$} ({pattern}), {src}, {index}", "match")
            }
            Instruction::CheckMatchGuard { src } => {
                write!(f, "{:WIDTH$} {src}", "check-match-guard")
            }
            Instruction::Iterate {
                dst,
                stream,
                end_index,
            } => {
                write!(f, "{:WIDTH$} {dst}, {stream}, end {end_index}", "iterate")
            }
            Instruction::OnError { index } => {
                write!(f, "{:WIDTH$} {index}", "on-error")
            }
            Instruction::OnErrorInto { index, dst } => {
                write!(f, "{:WIDTH$} {index}, {dst}", "on-error-into")
            }
            Instruction::PopErrorHandler => {
                write!(f, "{:WIDTH$}", "pop-error-handler")
            }
            Instruction::ReturnEarly { src } => {
                write!(f, "{:WIDTH$} {src}", "return-early")
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
        write!(f, "decl {} {:?}", self.0.get(), self.1)
    }
}

struct FmtVar<'a>(VarId, Option<&'a str>);

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
            write!(f, "var {} {:?}", self.0.get(), name)
        } else {
            write!(f, "var {}", self.0.get())
        }
    }
}

impl fmt::Display for RedirectMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RedirectMode::Pipe => write!(f, "pipe"),
            RedirectMode::PipeSeparate => write!(f, "pipe separate"),
            RedirectMode::Value => write!(f, "value"),
            RedirectMode::Null => write!(f, "null"),
            RedirectMode::Inherit => write!(f, "inherit"),
            RedirectMode::Print => write!(f, "print"),
            RedirectMode::File { file_num } => write!(f, "file({file_num})"),
            RedirectMode::Caller => write!(f, "caller"),
        }
    }
}

struct FmtData<'a>(&'a [u8], DataSlice);

impl fmt::Display for FmtData<'_> {
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

impl fmt::Display for FmtLiteral<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.literal {
            Literal::Bool(b) => write!(f, "bool({b:?})"),
            Literal::Int(i) => write!(f, "int({i:?})"),
            Literal::Float(fl) => write!(f, "float({fl:?})"),
            Literal::Filesize(q) => write!(f, "filesize({q}b)"),
            Literal::Duration(q) => write!(f, "duration({q}ns)"),
            Literal::Binary(b) => write!(f, "binary({})", FmtData(self.data, *b)),
            Literal::Block(id) => write!(f, "block({})", id.get()),
            Literal::Closure(id) => write!(f, "closure({})", id.get()),
            Literal::RowCondition(id) => write!(f, "row_condition({})", id.get()),
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
            Literal::Date(dt) => write!(f, "date({dt})"),
            Literal::Nothing => write!(f, "nothing"),
        }
    }
}

struct FmtPattern<'a> {
    engine_state: &'a EngineState,
    pattern: &'a Pattern,
}

impl fmt::Display for FmtPattern<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pattern {
            Pattern::Record(bindings) => {
                f.write_str("{")?;
                for (name, pattern) in bindings {
                    write!(
                        f,
                        "{}: {}",
                        name,
                        FmtPattern {
                            engine_state: self.engine_state,
                            pattern: &pattern.pattern,
                        }
                    )?;
                }
                f.write_str("}")
            }
            Pattern::List(bindings) => {
                f.write_str("[")?;
                for pattern in bindings {
                    write!(
                        f,
                        "{}",
                        FmtPattern {
                            engine_state: self.engine_state,
                            pattern: &pattern.pattern
                        }
                    )?;
                }
                f.write_str("]")
            }
            Pattern::Expression(expr) => {
                let string =
                    String::from_utf8_lossy(self.engine_state.get_span_contents(expr.span));
                f.write_str(&string)
            }
            Pattern::Value(value) => {
                f.write_str(&value.to_parsable_string(", ", &self.engine_state.config))
            }
            Pattern::Variable(var_id) => {
                let variable = FmtVar::new(self.engine_state, *var_id);
                write!(f, "{variable}")
            }
            Pattern::Or(patterns) => {
                for (index, pattern) in patterns.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" | ")?;
                    }
                    write!(
                        f,
                        "{}",
                        FmtPattern {
                            engine_state: self.engine_state,
                            pattern: &pattern.pattern
                        }
                    )?;
                }
                Ok(())
            }
            Pattern::Rest(var_id) => {
                let variable = FmtVar::new(self.engine_state, *var_id);
                write!(f, "..{variable}")
            }
            Pattern::IgnoreRest => f.write_str(".."),
            Pattern::IgnoreValue => f.write_str("_"),
            Pattern::Garbage => f.write_str("<garbage>"),
        }
    }
}
