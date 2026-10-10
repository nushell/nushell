//! Bridge: run Nushell scripts through the real engine using
//! `nu-winnow-parser` as the front end.
//!
//! This is an MVP of how the parser plugs into Nushell. The pipeline is:
//!
//! 1. `nu_winnow_parser::parse` produces the syntactic AST.
//! 2. [`Lower`] walks that AST and builds `nu_protocol::ast` nodes inside a
//!    `StateWorkingSet`: it resolves command names to `DeclId`s and applies
//!    each command's signature (which flags take values, which positional is a
//!    block), declares variables and custom commands, tracks closure captures,
//!    and registers spans. This is exactly the work `nu-parser` interleaves with
//!    lexing today; here it is a separate pass over a finished tree.
//! 3. `nu_parser::compile_block` compiles the block to IR and `nu_engine::eval_block`
//!    runs it.
//!
//! ```text
//! bridge 'ls | where size > 1kb | length'   # run a script through the bridge
//! bridge --file script.nu
//! bridge --compare 'script'                 # also run it through nu-parser, compare results
//! bridge --demo                             # a built-in suite of scripts run both ways
//! ```
//!
//! Supported: literals, lists, records, tables, ranges, strings and
//! interpolation, variables and cell paths, math and boolean operators, calls
//! with signature-driven flags and positionals, spreads, closures, blocks,
//! subexpressions, external calls, `let`/`mut`, assignments, `if`/`else`,
//! `match`, `for`, `while`, `loop`, `break`/`continue`/`return`, `try`/`catch`/
//! `finally`, `where` row conditions and `def`. Not yet lowered: `use`,
//! `module`, `export`, `alias`, `extern`, `const`, attributes, redirections and
//! environment shorthand (all of which need the module/overlay machinery of
//! `nu-parser`).

use std::collections::HashSet;
use std::sync::Arc;

use nu_protocol::ast::{
    Assignment, Bits, Block, Boolean, Call, CellPath, Comparison, Expr, Expression, ExternalArgument, FullCellPath,
    Keyword, ListItem, MatchPattern, Math, Operator, PathMember, Pattern, Pipeline, PipelineElement, Range,
    RangeInclusion, RangeOperator, RecordItem, Table, Unit, ValueWithUnit,
};
use nu_protocol::casing::Casing;
use nu_protocol::debugger::WithoutDebug;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    BlockId, ENV_VARIABLE_ID, FilesizeUnit, Flag, IN_VARIABLE_ID, NU_VARIABLE_ID, PipelineData, PositionalArg,
    Signature, Span, Spanned, SyntaxShape, Type, Value, VarId,
};
use nu_winnow_parser::ast as w;
use nu_winnow_parser::lex::AssignmentOperator;
use nushell_harness::engine;

type LResult<T> = Result<T, String>;

/// Lowers the syntactic AST into the engine's AST.
struct Lower<'ws, 'e, 'a> {
    ws: &'ws mut StateWorkingSet<'e>,
    src: &'a str,
    /// Offset of the source text inside the working set's file table.
    offset: usize,
    /// One frame per open closure: the variables declared inside it and the
    /// outer variables it uses (its captures).
    closures: Vec<ClosureFrame>,
}

#[derive(Default)]
struct ClosureFrame {
    declared: HashSet<VarId>,
    captures: Vec<(VarId, Span)>,
}

impl<'ws, 'e, 'a> Lower<'ws, 'e, 'a> {
    fn span(&self, s: nu_winnow_parser::Span) -> Span {
        Span::new(s.start + self.offset, s.end + self.offset)
    }

    /// The span of the keyword `kw` that starts the statement covering `span`.
    fn keyword_span(&self, span: Span, kw: &str) -> Span {
        Span::new(span.start, span.start + kw.len())
    }

    fn expression(&mut self, expr: Expr, span: Span, ty: Type) -> Expression {
        Expression::new(self.ws, expr, span, ty)
    }

    fn unsupported<T>(&self, what: &str, span: nu_winnow_parser::Span) -> LResult<T> {
        let text = span.slice(self.src);
        let text: String = text.chars().take(40).collect();
        Err(format!("`{what}` is not lowered by the bridge yet: `{text}`"))
    }

    // --- scopes and variables ---------------------------------------------------

    fn declare_var(&mut self, name: &str, span: Span, mutable: bool) -> VarId {
        let id = self.ws.add_variable(format!("${name}").into_bytes(), span, Type::Any, mutable);
        if let Some(frame) = self.closures.last_mut() {
            frame.declared.insert(id);
        }
        id
    }

    fn resolve_var(&mut self, name: &str, span: Span) -> LResult<VarId> {
        let id = match name {
            "nu" => NU_VARIABLE_ID,
            "in" => IN_VARIABLE_ID,
            "env" => ENV_VARIABLE_ID,
            _ => self
                .ws
                .find_variable(format!("${name}").as_bytes())
                .ok_or_else(|| format!("variable `${name}` not found"))?,
        };
        // Record captures for every enclosing closure that does not declare the variable.
        if !matches!(id, NU_VARIABLE_ID | IN_VARIABLE_ID | ENV_VARIABLE_ID) {
            for frame in self.closures.iter_mut().rev() {
                if frame.declared.contains(&id) {
                    break;
                }
                if !frame.captures.iter().any(|(v, _)| *v == id) {
                    frame.captures.push((id, span));
                }
            }
        }
        Ok(id)
    }

    // --- blocks -------------------------------------------------------------------------

    /// Lower a block's pipelines into `block`.
    fn fill_block(&mut self, block: &w::Block<'a>, out: &mut Block) -> LResult<()> {
        for pipeline in &block.pipelines {
            let mut elements = Vec::with_capacity(pipeline.elements.len());
            for element in &pipeline.elements {
                if element.redirection.is_some() {
                    return self.unsupported("redirection", element.span);
                }
                let expr = self.expr(&element.expr)?;
                elements.push(PipelineElement { pipe: element.pipe.map(|p| self.span(p)), expr, redirection: None });
            }
            out.pipelines.push(Pipeline { elements });
        }
        Ok(())
    }

    /// A scoped block (bodies of `if`, `for`, closures, ...), lowered and added to the working set.
    /// A keyword's block argument, or the variable or subexpression standing
    /// in for it (`if $c $env.f`), which nu type-checks as a block.
    fn block_arg(&mut self, block: &w::Block<'a>, value: Option<&w::Expression<'a>>) -> LResult<Expression> {
        if let Some(value) = value {
            return self.expr(value);
        }
        let block_span = self.span(block.span);
        let id = self.scoped_block(block, block_span, false)?;
        Ok(self.expression(Expr::Block(id), block_span, Type::Block))
    }

    fn scoped_block(&mut self, block: &w::Block<'a>, span: Span, to_ir: bool) -> LResult<BlockId> {
        self.ws.enter_scope();
        let mut out = Block::new();
        out.span = Some(span);
        let result = self.fill_block(block, &mut out);
        self.ws.exit_scope();
        result?;
        if to_ir {
            compile(self.ws, &mut out)?;
        }
        Ok(self.ws.add_block(Arc::new(out)))
    }

    /// A closure or command body: parameters become variables in a fresh scope,
    /// captures are tracked, and the block is compiled to IR.
    fn closure_block(
        &mut self,
        params: Option<&w::Signature<'a>>,
        body: &w::Block<'a>,
        span: Span,
        name: &str,
    ) -> LResult<(BlockId, Signature)> {
        self.ws.enter_scope();
        self.closures.push(ClosureFrame::default());
        let mut sig = Signature::new(name);
        if let Some(params) = params {
            for p in &params.params {
                let pspan = self.span(p.span);
                let var_id = self.declare_var(p.name.item, pspan, false);
                let shape = p.ty.as_ref().map_or(SyntaxShape::Any, |t| shape_of(&t.shape));
                let default_value = match &p.default {
                    Some(d) => {
                        let e = self.expr(d)?;
                        Some(nu_protocol::eval_const::eval_constant(self.ws, &e).map_err(|e| e.to_string())?)
                    }
                    None => None,
                };
                match &p.kind {
                    w::ParameterKind::Required | w::ParameterKind::Optional => {
                        let arg = PositionalArg {
                            name: p.name.item.to_string(),
                            desc: String::new(),
                            shape,
                            completion: None,
                            var_id: Some(var_id),
                            default_value,
                        };
                        if p.kind == w::ParameterKind::Optional {
                            sig.optional_positional.push(arg);
                        } else {
                            sig.required_positional.push(arg);
                        }
                    }
                    w::ParameterKind::Rest => {
                        sig.rest_positional = Some(PositionalArg {
                            name: p.name.item.to_string(),
                            desc: String::new(),
                            shape,
                            completion: None,
                            var_id: Some(var_id),
                            default_value,
                        });
                    }
                    w::ParameterKind::Flag { long, short } => {
                        sig.named.push(Flag {
                            long: long.map(|l| l.item.to_string()).unwrap_or_default(),
                            short: short.map(|s| s.item),
                            arg: p.ty.as_ref().map(|t| shape_of(&t.shape)),
                            required: false,
                            desc: String::new(),
                            completion: None,
                            var_id: Some(var_id),
                            default_value,
                        });
                    }
                }
            }
        }
        let mut out = Block::new();
        out.span = Some(span);
        let result = self.fill_block(body, &mut out);
        let frame = self.closures.pop().expect("frame");
        self.ws.exit_scope();
        result?;
        out.captures = frame.captures.clone();
        // Captures of an inner closure that the enclosing closure does not declare are its captures too.
        if let Some(outer) = self.closures.last_mut() {
            for (id, span) in frame.captures {
                if !outer.declared.contains(&id) && !outer.captures.iter().any(|(v, _)| *v == id) {
                    outer.captures.push((id, span));
                }
            }
        }
        out.signature = Box::new(sig.clone());
        compile(self.ws, &mut out)?;
        Ok((self.ws.add_block(Arc::new(out)), sig))
    }

    // --- expressions ----------------------------------------------------------------------

    fn expr(&mut self, e: &w::Expression<'a>) -> LResult<Expression> {
        let span = self.span(e.span);
        let (expr, ty) = match &e.expr {
            w::Expr::Bool(b) => (Expr::Bool(*b), Type::Bool),
            w::Expr::Nothing => (Expr::Nothing, Type::Nothing),
            w::Expr::Int(i) => (Expr::Int(*i), Type::Int),
            w::Expr::Float(f) => (Expr::Float(*f), Type::Float),
            w::Expr::String(s) => match s.quote {
                w::Quote::Raw(_) => (Expr::RawString(s.value.to_string()), Type::String),
                _ => (Expr::String(s.value.to_string()), Type::String),
            },
            w::Expr::StringInterpolation(i) => {
                let mut parts = Vec::with_capacity(i.parts.len());
                for part in &i.parts {
                    parts.push(match part {
                        w::InterpolationPart::Text { span, value } => {
                            let sp = self.span(*span);
                            self.expression(Expr::String(value.to_string()), sp, Type::String)
                        }
                        w::InterpolationPart::Expression(e) => self.expr(e)?,
                    });
                }
                (Expr::StringInterpolation(parts), Type::String)
            }
            w::Expr::Binary(b) => (Expr::Binary(b.bytes.clone()), Type::Binary),
            w::Expr::Duration(d) => {
                let ns = d.to_nanoseconds().ok_or("duration too large")?;
                let inner = self.expression(Expr::Int(ns), span, Type::Int);
                (
                    Expr::ValueWithUnit(Box::new(ValueWithUnit {
                        expr: inner,
                        unit: Spanned { item: Unit::Nanosecond, span },
                    })),
                    Type::Duration,
                )
            }
            w::Expr::Filesize(f) => {
                let bytes = f.to_bytes().ok_or("filesize too large")?;
                let inner = self.expression(Expr::Int(bytes), span, Type::Int);
                (
                    Expr::ValueWithUnit(Box::new(ValueWithUnit {
                        expr: inner,
                        unit: Spanned { item: Unit::Filesize(FilesizeUnit::B), span },
                    })),
                    Type::Filesize,
                )
            }
            w::Expr::DateTime(text) => {
                let dt = chrono::DateTime::parse_from_rfc3339(text)
                    .or_else(|_| chrono::DateTime::parse_from_rfc3339(&format!("{text}T00:00:00+00:00")))
                    .or_else(|_| chrono::DateTime::parse_from_rfc3339(&format!("{text}+00:00")))
                    .map_err(|e| format!("invalid datetime `{text}`: {e}"))?;
                (Expr::DateTime(dt), Type::Date)
            }
            w::Expr::Range(r) => {
                let from = r.from.as_ref().map(|e| self.expr(e)).transpose()?;
                let next = r.next.as_ref().map(|e| self.expr(e)).transpose()?;
                let to = r.to.as_ref().map(|e| self.expr(e)).transpose()?;
                let inclusion = match r.operator.inclusion {
                    w::RangeInclusion::Inclusive => RangeInclusion::Inclusive,
                    w::RangeInclusion::RightExclusive => RangeInclusion::RightExclusive,
                };
                let operator = RangeOperator {
                    inclusion,
                    span: self.span(r.operator.span),
                    next_op_span: r.operator.next_op_span.map_or(span, |s| self.span(s)),
                };
                (Expr::Range(Box::new(Range { from, next, to, operator })), Type::Range)
            }
            w::Expr::Var(v) => (Expr::Var(self.resolve_var(v.name, span)?), Type::Any),
            w::Expr::CellPath(c) => {
                (Expr::CellPath(CellPath { members: self.members(&c.members) }), Type::CellPath)
            }
            w::Expr::FullCellPath(p) => {
                let head = self.expr(&p.head)?;
                let tail = self.members(&p.tail);
                (Expr::FullCellPath(Box::new(FullCellPath { head, tail })), Type::Any)
            }
            w::Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(match item {
                        w::ListItem::Item(e) => ListItem::Item(self.expr(e)?),
                        w::ListItem::Spread { dots, expr } => ListItem::Spread(self.span(*dots), self.expr(expr)?),
                    });
                }
                (Expr::List(out), Type::list(Type::Any))
            }
            w::Expr::Table(t) => {
                let columns = self.list_items(&t.columns)?;
                let mut rows = Vec::with_capacity(t.rows.len());
                for row in &t.rows {
                    rows.push(self.list_items(row)?.into_boxed_slice());
                }
                (
                    Expr::Table(Table { columns: columns.into_boxed_slice(), rows: rows.into_boxed_slice() }),
                    Type::table(),
                )
            }
            w::Expr::Record(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(match item {
                        w::RecordItem::Pair { key, value, .. } => RecordItem::Pair(self.expr(key)?, self.expr(value)?),
                        w::RecordItem::Spread { dots, expr } => RecordItem::Spread(self.span(*dots), self.expr(expr)?),
                    });
                }
                (Expr::Record(out), Type::record())
            }
            w::Expr::Closure(c) => {
                let (id, _) = self.closure_block(c.params.as_ref(), &c.body, span, "closure")?;
                (Expr::Closure(id), Type::Closure)
            }
            w::Expr::Block(b) => (Expr::Block(self.scoped_block(b, span, false)?), Type::Block),
            w::Expr::Subexpression(b) => (Expr::Subexpression(self.scoped_block(b, span, false)?), Type::Any),
            w::Expr::BinaryOp(b) => {
                let lhs = self.expr(&b.lhs)?;
                let op = self.expression(Expr::Operator(operator(b.op.item)), self.span(b.op.span), Type::Any);
                let rhs = self.expr(&b.rhs)?;
                (Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)), Type::Any)
            }
            w::Expr::UnaryNot(n) => (Expr::UnaryNot(Box::new(self.expr(&n.expr)?)), Type::Bool),
            w::Expr::Assignment(a) => {
                let lhs = self.expr(&a.lhs)?;
                let op = self.expression(
                    Expr::Operator(Operator::Assignment(assign_op(a.op.item))),
                    self.span(a.op.span),
                    Type::Any,
                );
                let rhs_span = self.span(a.rhs.span);
                let rhs_id = self.scoped_block(&a.rhs, rhs_span, false)?;
                let rhs = self.expression(Expr::Subexpression(rhs_id), rhs_span, Type::Any);
                (Expr::BinaryOp(Box::new(lhs), Box::new(op), Box::new(rhs)), Type::Nothing)
            }
            w::Expr::Call(c) => return self.call(c, span),
            w::Expr::ExternalCall(c) => {
                let head = self.expr(&c.head)?;
                let mut args = Vec::with_capacity(c.arguments.len());
                for arg in &c.arguments {
                    args.push(match arg {
                        w::ExternalArgument::Regular(e) => ExternalArgument::Regular(self.expr(e)?),
                        w::ExternalArgument::Spread { expr, .. } => ExternalArgument::Spread(self.expr(expr)?),
                    });
                }
                (Expr::ExternalCall(Box::new(head), args.into_boxed_slice()), Type::Any)
            }
            w::Expr::Let(b) => return self.binding(b, "let", false, span),
            w::Expr::Mut(b) => return self.binding(b, "mut", true, span),
            w::Expr::Def(d) => return self.def(d, span),
            w::Expr::If(i) => return self.if_expr(i, span),
            w::Expr::Match(m) => return self.match_expr(m, span),
            w::Expr::For(f) => return self.for_expr(f, span),
            w::Expr::While(wh) => {
                let mut call = self.keyword_call("while", self.keyword_span(span, "while"))?;
                let cond = self.expr(&wh.condition)?;
                call.add_positional(cond);
                let body = self.block_arg(&wh.body, wh.body_value.as_deref())?;
                call.add_positional(body);
                (Expr::Call(Box::new(call)), Type::Nothing)
            }
            w::Expr::Loop(l) => {
                let mut call = self.keyword_call("loop", self.keyword_span(span, "loop"))?;
                let body = self.block_arg(&l.body, l.body_value.as_deref())?;
                call.add_positional(body);
                (Expr::Call(Box::new(call)), Type::Nothing)
            }
            w::Expr::Break => (Expr::Call(Box::new(self.keyword_call("break", span)?)), Type::Nothing),
            w::Expr::Continue => (Expr::Call(Box::new(self.keyword_call("continue", span)?)), Type::Nothing),
            w::Expr::Return(r) => {
                let mut call = self.keyword_call("return", self.keyword_span(span, "return"))?;
                if let Some(v) = &r.value {
                    let v = self.expr(v)?;
                    call.add_positional(v);
                }
                (Expr::Call(Box::new(call)), Type::Nothing)
            }
            w::Expr::Try(t) => {
                let mut call = self.keyword_call("try", self.keyword_span(span, "try"))?;
                let body = self.block_arg(&t.body, t.body_value.as_deref())?;
                call.add_positional(body);
                for h in &t.handlers {
                    let kw = match h.kind {
                        w::HandlerKind::Catch => "catch",
                        w::HandlerKind::Finally => "finally",
                    };
                    let handler = self.expr(&h.body)?;
                    let kw_span = self.span(h.keyword);
                    let hspan = kw_span.append(handler.span);
                    let keyword = Keyword { keyword: kw.as_bytes().into(), span: kw_span, expr: handler };
                    let arg = self.expression(Expr::Keyword(Box::new(keyword)), hspan, Type::Any);
                    call.add_positional(arg);
                }
                (Expr::Call(Box::new(call)), Type::Any)
            }
            w::Expr::Where(wh) => {
                let mut call = self.keyword_call("where", self.keyword_span(span, "where"))?;
                let cond_span = self.span(wh.condition.span);
                let block_id = match &wh.condition.expr {
                    w::Expr::Closure(c) => self.closure_block(c.params.as_ref(), &c.body, cond_span, "closure")?.0,
                    _ => {
                        // A row condition: a closure taking `$it`.
                        self.ws.enter_scope();
                        self.closures.push(ClosureFrame::default());
                        let it = self.declare_var("it", cond_span, false);
                        let result = self.expr(&wh.condition);
                        let frame = self.closures.pop().expect("frame");
                        self.ws.exit_scope();
                        let cond = result?;
                        let mut block = Block::new();
                        block.span = Some(cond_span);
                        block.pipelines.push(Pipeline::from_vec(vec![cond]));
                        block.captures = frame.captures;
                        block.signature.required_positional.push(PositionalArg {
                            name: "$it".into(),
                            desc: "row condition".into(),
                            shape: SyntaxShape::Any,
                            completion: None,
                            var_id: Some(it),
                            default_value: None,
                        });
                        compile(self.ws, &mut block)?;
                        self.ws.add_block(Arc::new(block))
                    }
                };
                let cond = self.expression(Expr::RowCondition(block_id), cond_span, Type::Bool);
                call.add_positional(cond);
                (Expr::Call(Box::new(call)), Type::Any)
            }
            w::Expr::Const(_) => return self.unsupported("const", e.span),
            w::Expr::Extern(_) => return self.unsupported("extern", e.span),
            w::Expr::Alias(_) => return self.unsupported("alias", e.span),
            w::Expr::Use(_) => return self.unsupported("use", e.span),
            w::Expr::Module(_) => return self.unsupported("module", e.span),
            w::Expr::Export(_) => return self.unsupported("export", e.span),
            w::Expr::ExportEnv(_) => return self.unsupported("export-env", e.span),
            w::Expr::EnvShorthand(_) => return self.unsupported("environment shorthand", e.span),
            w::Expr::AttributeBlock(_) => return self.unsupported("attributes", e.span),
            w::Expr::Garbage => return Err("cannot lower a garbage node".into()),
            _ => return self.unsupported("expression", e.span),
        };
        Ok(self.expression(expr, span, ty))
    }

    fn list_items(&mut self, list: &w::Expression<'a>) -> LResult<Vec<Expression>> {
        let w::Expr::List(items) = &list.expr else { return Err("table row must be a list".into()) };
        items
            .iter()
            .map(|item| match item {
                w::ListItem::Item(e) => self.expr(e),
                w::ListItem::Spread { .. } => Err("cannot spread inside a table row".into()),
            })
            .collect()
    }

    fn members(&self, members: &[w::PathMember<'a>]) -> Vec<PathMember> {
        members
            .iter()
            .map(|m| {
                let span = self.span(m.span);
                match &m.kind {
                    w::PathMemberKind::Int(i) => PathMember::int(*i, m.optional, span),
                    w::PathMemberKind::String(s) => PathMember::string(
                        s.to_string(),
                        m.optional,
                        if m.case_insensitive { Casing::Insensitive } else { Casing::Sensitive },
                        span,
                    ),
                }
            })
            .collect()
    }

    // --- calls --------------------------------------------------------------------------------

    fn keyword_call(&mut self, name: &str, head: Span) -> LResult<Call> {
        let decl_id = self
            .ws
            .find_decl(name.as_bytes())
            .ok_or_else(|| format!("keyword `{name}` is not registered in the engine"))?;
        let mut call = Call::new(head);
        call.decl_id = decl_id;
        Ok(call)
    }

    /// A call to an internal command, with arguments assigned by its signature.
    /// The parser already makes a head it does not know an `ExternalCall`; a
    /// name it knows but this engine does not still becomes one, as in Nushell.
    fn call(&mut self, c: &w::Call<'a>, span: Span) -> LResult<Expression> {
        let head_span = self.span(c.head.span);
        let Some(decl_id) = self.ws.find_decl(c.head.name.as_bytes()) else {
            return self.external_fallback(c, span);
        };
        let sig = self.ws.get_decl(decl_id).signature();
        let mut call = Call::new(head_span);
        call.decl_id = decl_id;
        let mut positional_idx = 0usize;
        let mut end_of_options = false;
        let mut args = c.arguments.iter().peekable();
        while let Some(arg) = args.next() {
            match arg {
                w::Argument::EndOfOptions(_) => end_of_options = true,
                w::Argument::Named(f) if !end_of_options => {
                    let flag_span = self.span(f.span);
                    let flags: Vec<(Flag, usize)> = if f.long {
                        let flag = sig
                            .get_long_flag(f.name)
                            .ok_or_else(|| format!("`{}` has no flag `--{}`", sig.name, f.name))?;
                        vec![(flag, f.name.len() + 2)]
                    } else {
                        let mut out = Vec::new();
                        for (i, ch) in f.name.char_indices() {
                            let flag =
                                sig.get_short_flag(ch).ok_or_else(|| format!("`{}` has no flag `-{ch}`", sig.name))?;
                            out.push((flag, i + 1 + ch.len_utf8()));
                        }
                        out
                    };
                    let count = flags.len();
                    for (i, (flag, _)) in flags.into_iter().enumerate() {
                        let last = i + 1 == count;
                        let value = if flag.arg.is_some() && last {
                            if let Some(v) = &f.value {
                                Some(self.expr(v)?)
                            } else {
                                match args.next() {
                                    Some(w::Argument::Positional(v)) => Some(self.expr_with_shape(v, flag.arg.as_ref())?),
                                    _ => return Err(format!("flag `--{}` needs a value", flag.long)),
                                }
                            }
                        } else {
                            None
                        };
                        let short = if f.long {
                            None
                        } else {
                            flag.short.map(|s| Spanned { item: s.to_string(), span: flag_span })
                        };
                        call.add_named((Spanned { item: flag.long.clone(), span: flag_span }, short, value));
                    }
                }
                w::Argument::Named(f) => {
                    let e = self.expression(
                        Expr::String(format!("-{}{}", if f.long { "-" } else { "" }, f.name)),
                        self.span(f.span),
                        Type::String,
                    );
                    call.add_positional(e);
                    positional_idx += 1;
                }
                w::Argument::Positional(e) => {
                    let shape = sig.get_positional(positional_idx).map(|p| p.shape.clone());
                    let e = self.expr_with_shape(e, shape.as_ref())?;
                    call.add_positional(e);
                    positional_idx += 1;
                }
                w::Argument::Spread { expr, .. } => {
                    let e = self.expr(expr)?;
                    call.add_spread(e);
                }
            }
        }
        Ok(self.expression(Expr::Call(Box::new(call)), span, Type::Any))
    }

    /// Lower an argument in the light of the shape its command declares: a
    /// `{ ... }` becomes a block rather than a closure for `Block` shapes.
    fn expr_with_shape(&mut self, e: &w::Expression<'a>, shape: Option<&SyntaxShape>) -> LResult<Expression> {
        let span = self.span(e.span);
        match (shape, &e.expr) {
            (Some(SyntaxShape::Block), w::Expr::Closure(c)) if c.params.is_none() => {
                let id = self.scoped_block(&c.body, span, false)?;
                Ok(self.expression(Expr::Block(id), span, Type::Block))
            }
            // `get name.0`: a bare word in a cell-path position is a cell path.
            (Some(SyntaxShape::CellPath), w::Expr::String(s)) if s.quote == w::Quote::Bare => {
                let mut members = Vec::new();
                let mut offset = span.start;
                for part in s.value.split('.') {
                    let optional = part.ends_with('?');
                    let name = part.trim_end_matches('?');
                    let pspan = Span::new(offset, offset + part.len());
                    members.push(match name.parse::<usize>() {
                        Ok(i) => PathMember::int(i, optional, pspan),
                        Err(_) => PathMember::string(name.to_string(), optional, Casing::Sensitive, pspan),
                    });
                    offset += part.len() + 1;
                }
                Ok(self.expression(Expr::CellPath(CellPath { members }), span, Type::CellPath))
            }
            _ => self.expr(e),
        }
    }

    fn external_fallback(&mut self, c: &w::Call<'a>, span: Span) -> LResult<Expression> {
        let head_span = self.span(c.head.span);
        let head = self.expression(Expr::String(c.head.name.to_string()), head_span, Type::String);
        let mut args = Vec::new();
        for arg in &c.arguments {
            args.push(match arg {
                w::Argument::Positional(e) => ExternalArgument::Regular(self.expr(e)?),
                w::Argument::Named(f) => {
                    let text = f.span.slice(self.src).to_string();
                    ExternalArgument::Regular(self.expression(Expr::String(text), self.span(f.span), Type::String))
                }
                w::Argument::Spread { expr, .. } => ExternalArgument::Spread(self.expr(expr)?),
                w::Argument::EndOfOptions(s) => {
                    ExternalArgument::Regular(self.expression(Expr::String("--".into()), self.span(*s), Type::String))
                }
            });
        }
        Ok(self.expression(Expr::ExternalCall(Box::new(head), args.into_boxed_slice()), span, Type::Any))
    }

    // --- keyword statements -------------------------------------------------------------

    fn binding(&mut self, b: &w::Binding<'a>, keyword: &str, mutable: bool, span: Span) -> LResult<Expression> {
        let mut call = self.keyword_call(keyword, self.keyword_span(span, keyword))?;
        let value = match &b.value {
            Some(v) => {
                let vspan = self.span(v.span);
                let id = self.scoped_block(v, vspan, false)?;
                Some(self.expression(Expr::Block(id), vspan, Type::Any))
            }
            None => None,
        };
        // The variable is declared after its value is lowered, so `let x = $x + 1` sees the outer `x`.
        let var_id = self.declare_var(b.name.item, self.span(b.name.span), mutable);
        let decl = self.expression(Expr::VarDecl(var_id), self.span(b.name.span), Type::Any);
        call.add_positional(decl);
        if let Some(value) = value {
            call.add_positional(value);
        }
        Ok(self.expression(Expr::Call(Box::new(call)), span, Type::Nothing))
    }

    /// `def`: the command is registered *before* its body is lowered (with a
    /// placeholder block that is filled in afterwards) so that the body can call
    /// the command recursively, as `nu-parser`'s predeclaration pass allows.
    fn def(&mut self, d: &w::Def<'a>, span: Span) -> LResult<Expression> {
        let body_span = self.span(d.body.span);
        // Parameter variables are created now but only enter scope inside the body.
        let mut sig = Signature::new(d.name.item.to_string());
        let mut params: Vec<(String, VarId)> = Vec::new();
        for p in &d.signature.params {
            let pspan = self.span(p.span);
            let var_id = self.ws.add_variable_without_scope(pspan, Type::Any, false);
            params.push((format!("${}", p.name.item), var_id));
            let shape = p.ty.as_ref().map_or(SyntaxShape::Any, |t| shape_of(&t.shape));
            let default_value = match &p.default {
                Some(dflt) => {
                    let e = self.expr(dflt)?;
                    Some(nu_protocol::eval_const::eval_constant(self.ws, &e).map_err(|e| e.to_string())?)
                }
                None => None,
            };
            let positional = |name: &str, shape: SyntaxShape, default_value: Option<Value>| PositionalArg {
                name: name.to_string(),
                desc: String::new(),
                shape,
                completion: None,
                var_id: Some(var_id),
                default_value,
            };
            match &p.kind {
                w::ParameterKind::Required => {
                    sig.required_positional.push(positional(p.name.item, shape, default_value))
                }
                w::ParameterKind::Optional => {
                    sig.optional_positional.push(positional(p.name.item, shape, default_value))
                }
                w::ParameterKind::Rest => sig.rest_positional = Some(positional(p.name.item, shape, default_value)),
                w::ParameterKind::Flag { long, short } => sig.named.push(Flag {
                    long: long.map(|l| l.item.to_string()).unwrap_or_default(),
                    short: short.map(|s| s.item),
                    arg: p.ty.as_ref().map(|t| shape_of(&t.shape)),
                    required: false,
                    desc: String::new(),
                    completion: None,
                    var_id: Some(var_id),
                    default_value,
                }),
            }
        }
        let placeholder = self.ws.add_block(Arc::new(Block::new()));
        self.ws.add_decl(sig.clone().into_block_command(placeholder, Vec::new(), Vec::new()));

        self.ws.enter_scope();
        self.closures.push(ClosureFrame::default());
        for (name, id) in &params {
            self.ws.insert_variable_into_scope(name.clone().into_bytes(), *id);
            self.closures.last_mut().expect("frame").declared.insert(*id);
        }
        let mut body = Block::new();
        body.span = Some(body_span);
        let result = self.fill_block(&d.body, &mut body);
        let frame = self.closures.pop().expect("frame");
        self.ws.exit_scope();
        result?;
        body.captures = frame.captures;
        body.signature = Box::new(sig);
        compile(self.ws, &mut body)?;
        *self.ws.get_block_mut(placeholder) = body;

        // The `def` statement itself evaluates to nothing; the compiler ignores its arguments.
        let mut call = self.keyword_call("def", self.keyword_span(span, "def"))?;
        let name = self.expression(Expr::String(d.name.item.to_string()), self.span(d.name.span), Type::String);
        call.add_positional(name);
        Ok(self.expression(Expr::Call(Box::new(call)), span, Type::Nothing))
    }

    fn if_expr(&mut self, i: &w::If<'a>, span: Span) -> LResult<Expression> {
        let mut call = self.keyword_call("if", self.keyword_span(span, "if"))?;
        let cond = self.expr(&i.condition)?;
        call.add_positional(cond);
        let then = self.block_arg(&i.then_block, i.then_value.as_deref())?;
        call.add_positional(then);
        if let Some(e) = &i.else_branch {
            let body = self.expr(&e.body)?;
            let kw_span = self.span(e.keyword);
            let full = kw_span.append(body.span);
            let keyword = Keyword { keyword: b"else".as_slice().into(), span: kw_span, expr: body };
            let arg = self.expression(Expr::Keyword(Box::new(keyword)), full, Type::Any);
            call.add_positional(arg);
        }
        Ok(self.expression(Expr::Call(Box::new(call)), span, Type::Any))
    }

    fn for_expr(&mut self, f: &w::For<'a>, span: Span) -> LResult<Expression> {
        let mut call = self.keyword_call("for", self.keyword_span(span, "for"))?;
        let iterable = self.expr(&f.iterable)?;
        self.ws.enter_scope();
        let var_id = self.declare_var(f.var.item, self.span(f.var.span), false);
        let decl = self.expression(Expr::VarDecl(var_id), self.span(f.var.span), Type::Any);
        call.add_positional(decl);
        let in_span = self.span(f.in_keyword);
        let full = in_span.append(iterable.span);
        let keyword = Keyword { keyword: b"in".as_slice().into(), span: in_span, expr: iterable };
        let kw = self.expression(Expr::Keyword(Box::new(keyword)), full, Type::Any);
        call.add_positional(kw);
        let body = self.block_arg(&f.body, f.body_value.as_deref());
        self.ws.exit_scope();
        call.add_positional(body?);
        Ok(self.expression(Expr::Call(Box::new(call)), span, Type::Nothing))
    }

    fn match_expr(&mut self, m: &w::Match<'a>, span: Span) -> LResult<Expression> {
        let mut call = self.keyword_call("match", self.keyword_span(span, "match"))?;
        let value = self.expr(&m.value)?;
        call.add_positional(value);
        let mut arms = Vec::with_capacity(m.arms.len());
        for arm in &m.arms {
            self.ws.enter_scope();
            let result = (|| {
                let mut pattern = self.pattern(&arm.pattern)?;
                if let Some(g) = &arm.guard {
                    pattern.guard = Some(Box::new(self.expr(g)?));
                }
                let body = self.expr(&arm.body)?;
                Ok::<_, String>((pattern, body))
            })();
            self.ws.exit_scope();
            arms.push(result?);
        }
        let block = self.expression(Expr::MatchBlock(arms), self.span(m.block_span), Type::Any);
        call.add_positional(block);
        Ok(self.expression(Expr::Call(Box::new(call)), span, Type::Any))
    }

    fn pattern(&mut self, p: &w::MatchPattern<'a>) -> LResult<MatchPattern> {
        let span = self.span(p.span);
        let pattern = match &p.pattern {
            w::Pattern::Expression(e) => Pattern::Expression(Box::new(self.expr(e)?)),
            w::Pattern::Variable(name) => Pattern::Variable(self.declare_var(name, span, false)),
            w::Pattern::IgnoreValue => Pattern::IgnoreValue,
            w::Pattern::List(items) => {
                Pattern::List(items.iter().map(|i| self.pattern(i)).collect::<LResult<_>>()?)
            }
            w::Pattern::Record(fields) => {
                let mut out = Vec::with_capacity(fields.len());
                for (name, pat) in fields {
                    out.push((name.item.to_string(), self.pattern(pat)?));
                }
                Pattern::Record(out)
            }
            w::Pattern::Rest(name) => Pattern::Rest(self.declare_var(name.item, span, false)),
            w::Pattern::IgnoreRest => Pattern::IgnoreRest,
            w::Pattern::Or(alts) => Pattern::Or(alts.iter().map(|a| self.pattern(a)).collect::<LResult<_>>()?),
        };
        Ok(MatchPattern { pattern, guard: None, span })
    }
}

/// Compile a block to IR so the engine can run it.
fn compile(ws: &mut StateWorkingSet<'_>, block: &mut Block) -> LResult<()> {
    match nu_engine::compile(ws, block) {
        Ok(ir) => {
            block.ir_block = Some(ir);
            Ok(())
        }
        Err(e) => Err(format!("compile error: {e:?}")),
    }
}

fn shape_of(kind: &w::SyntaxShape<'_>) -> SyntaxShape {
    match kind {
        w::SyntaxShape::Int => SyntaxShape::Int,
        w::SyntaxShape::Float => SyntaxShape::Float,
        w::SyntaxShape::Number => SyntaxShape::Number,
        w::SyntaxShape::String => SyntaxShape::String,
        w::SyntaxShape::Boolean => SyntaxShape::Boolean,
        w::SyntaxShape::Closure => SyntaxShape::Closure(None),
        w::SyntaxShape::Record(_) => SyntaxShape::record(),
        w::SyntaxShape::List(_) => SyntaxShape::List(Box::new(SyntaxShape::Any)),
        w::SyntaxShape::Table(_) => SyntaxShape::table(),
        w::SyntaxShape::Filepath => SyntaxShape::Filepath,
        w::SyntaxShape::GlobPattern => SyntaxShape::GlobPattern,
        w::SyntaxShape::Duration => SyntaxShape::Duration,
        w::SyntaxShape::Filesize => SyntaxShape::Filesize,
        w::SyntaxShape::DateTime => SyntaxShape::DateTime,
        w::SyntaxShape::Range => SyntaxShape::Range,
        w::SyntaxShape::CellPath => SyntaxShape::CellPath,
        w::SyntaxShape::Binary => SyntaxShape::Binary,
        w::SyntaxShape::Nothing => SyntaxShape::Nothing,
        _ => SyntaxShape::Any,
    }
}

fn operator(op: w::Operator) -> Operator {
    match op {
        w::Operator::Math(m) => Operator::Math(match m {
            w::Math::Add => Math::Add,
            w::Math::Subtract => Math::Subtract,
            w::Math::Multiply => Math::Multiply,
            w::Math::Divide => Math::Divide,
            w::Math::FloorDivide => Math::FloorDivide,
            w::Math::Modulo => Math::Modulo,
            w::Math::Pow => Math::Pow,
            w::Math::Concatenate => Math::Concatenate,
        }),
        w::Operator::Comparison(c) => Operator::Comparison(match c {
            w::Comparison::Equal => Comparison::Equal,
            w::Comparison::NotEqual => Comparison::NotEqual,
            w::Comparison::LessThan => Comparison::LessThan,
            w::Comparison::LessThanOrEqual => Comparison::LessThanOrEqual,
            w::Comparison::GreaterThan => Comparison::GreaterThan,
            w::Comparison::GreaterThanOrEqual => Comparison::GreaterThanOrEqual,
            w::Comparison::RegexMatch => Comparison::RegexMatch,
            w::Comparison::NotRegexMatch => Comparison::NotRegexMatch,
            w::Comparison::In => Comparison::In,
            w::Comparison::NotIn => Comparison::NotIn,
            w::Comparison::Has => Comparison::Has,
            w::Comparison::NotHas => Comparison::NotHas,
            w::Comparison::StartsWith => Comparison::StartsWith,
            w::Comparison::NotStartsWith => Comparison::NotStartsWith,
            w::Comparison::EndsWith => Comparison::EndsWith,
            w::Comparison::NotEndsWith => Comparison::NotEndsWith,
        }),
        w::Operator::Boolean(b) => Operator::Boolean(match b {
            w::Boolean::And => Boolean::And,
            w::Boolean::Or => Boolean::Or,
            w::Boolean::Xor => Boolean::Xor,
        }),
        w::Operator::Bits(b) => Operator::Bits(match b {
            w::Bits::BitOr => Bits::BitOr,
            w::Bits::BitXor => Bits::BitXor,
            w::Bits::BitAnd => Bits::BitAnd,
            w::Bits::ShiftLeft => Bits::ShiftLeft,
            w::Bits::ShiftRight => Bits::ShiftRight,
        }),
    }
}

fn assign_op(op: AssignmentOperator) -> Assignment {
    match op {
        AssignmentOperator::Assign => Assignment::Assign,
        AssignmentOperator::AddAssign => Assignment::AddAssign,
        AssignmentOperator::SubtractAssign => Assignment::SubtractAssign,
        AssignmentOperator::MultiplyAssign => Assignment::MultiplyAssign,
        AssignmentOperator::DivideAssign => Assignment::DivideAssign,
        AssignmentOperator::ConcatenateAssign => Assignment::ConcatenateAssign,
    }
}

// --- driving the engine -------------------------------------------------------------------

fn new_stack() -> Stack {
    let mut stack = Stack::new();
    for (k, v) in std::env::vars() {
        stack.add_env_var(k, Value::string(v, Span::unknown()));
    }
    if let Ok(cwd) = std::env::current_dir() {
        stack.add_env_var("PWD".into(), Value::string(cwd.to_string_lossy(), Span::unknown()));
    }
    stack
}

/// Parse with `nu-winnow-parser`, lower, compile and evaluate.
fn run_bridge(engine_state: &mut EngineState, src: &str) -> Result<Value, String> {
    let ast = nu_winnow_parser::parse(src).map_err(|e| e.render(src, Some("<bridge>")))?;
    let mut ws = StateWorkingSet::new(engine_state);
    let file_id = ws.add_file("<bridge>", src.as_bytes());
    let offset = ws.get_span_for_file(file_id).start;
    let mut lower = Lower { ws: &mut ws, src, offset, closures: Vec::new() };
    let mut block = Block::new();
    block.span = Some(Span::new(offset, offset + src.len()));
    lower.fill_block(&ast.block, &mut block)?;
    compile(&mut ws, &mut block)?;
    engine_state.merge_delta(ws.render()).map_err(|e| e.to_string())?;
    evaluate(engine_state, &block)
}

/// Parse with `nu-parser`, compile and evaluate: the reference path.
fn run_reference(engine_state: &mut EngineState, src: &str) -> Result<Value, String> {
    let mut ws = StateWorkingSet::new(engine_state);
    let block = nu_parser::parse(&mut ws, Some("<reference>"), src.as_bytes(), false);
    if let Some(err) = ws.parse_errors.first() {
        return Err(format!("parse error: {err:?}"));
    }
    engine_state.merge_delta(ws.render()).map_err(|e| e.to_string())?;
    evaluate(engine_state, &block)
}

fn evaluate(engine_state: &EngineState, block: &Block) -> Result<Value, String> {
    let mut stack = new_stack();
    let data = nu_engine::eval_block::<WithoutDebug>(engine_state, &mut stack, block, PipelineData::empty())
        .map_err(|e| e.to_string())?;
    data.body.into_value(Span::unknown()).map_err(|e| e.to_string())
}

const DEMO: &[&str] = &[
    "1 + 2 * 3",
    "[1 2 3] | each {|x| $x * 2 } | math sum",
    "let x = 10; let y = $x + 5; $y",
    "mut n = 0; for i in 1..4 { $n += $i }; $n",
    "{a: 1, b: [1 2 3]}.b.1",
    "[[name size]; [a 1kb] [b 3kb]] | where size > 2kb | get name.0",
    "if 5 > 3 { 'yes' } else { 'no' }",
    "match [1 2 3] { [$first, ..$rest] => $rest, _ => null }",
    "def double [x: int] { $x * 2 }; double 21",
    "def greet [name: string, --loud(-l)] { if $loud { $\"HELLO ($name | str upcase)\" } else { $\"hello ($name)\" } }; greet world -l",
    "let items = [3 1 2]; $items | sort | first",
    "1..5 | where {|n| $n mod 2 == 0 } | reverse",
    "try { error make {msg: boom} } catch {|e| $e.msg }",
    "$\"(2 + 2) is four\" | str upcase",
    "2024-01-02 | date to-timezone utc | format date '%Y'",
    "let r = {x: 1}; [1 2] | each {|i| $i + $r.x } | str join ','",
    "mut total = 0; while $total < 10 { $total += 3 }; $total",
    "not (1 == 2) and ('ab' starts-with 'a')",
    "0x[ff 00] | bytes length",
    "3sec + 500ms",
    "^echo hi there | str trim",
    "[a b c] | enumerate | where index > 0 | get item | str join",
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut compare = false;
    let mut demo = false;
    let mut file = None;
    let mut script = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--compare" => compare = true,
            "--demo" => demo = true,
            "--file" => file = it.next().cloned(),
            other => script = Some(other.to_string()),
        }
    }
    let mut engine_state = engine(true);
    if demo {
        let mut agree = 0;
        for src in DEMO {
            let bridge = run_bridge(&mut engine_state, src);
            let reference = run_reference(&mut engine_state, src);
            let same = match (&bridge, &reference) {
                (Ok(a), Ok(b)) => a == b,
                (Err(_), Err(_)) => true,
                _ => false,
            };
            agree += usize::from(same);
            let show = |r: &Result<Value, String>| match r {
                Ok(v) => v.to_expanded_string(", ", &engine_state.config),
                Err(e) => format!("error: {}", e.lines().next().unwrap_or("")),
            };
            println!(
                "{} {src}\n    bridge:    {}\n    nu-parser: {}",
                if same { "ok " } else { "!! " },
                show(&bridge),
                show(&reference)
            );
        }
        println!("{agree}/{} scripts produced identical results", DEMO.len());
        if agree != DEMO.len() {
            std::process::exit(1);
        }
        return;
    }
    let src = match (file, script) {
        (Some(path), _) => std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("cannot read {path}: {e}");
            std::process::exit(2)
        }),
        (None, Some(s)) => s,
        (None, None) => {
            eprintln!("usage: bridge [--compare] [--demo] [--file FILE] ['script']");
            std::process::exit(2)
        }
    };
    match run_bridge(&mut engine_state, &src) {
        Ok(v) => println!("{}", v.to_expanded_string("\n", &engine_state.config)),
        Err(e) => {
            eprintln!("bridge: {e}");
            std::process::exit(1)
        }
    }
    if compare {
        match run_reference(&mut engine_state, &src) {
            Ok(v) => println!("--- nu-parser:\n{}", v.to_expanded_string("\n", &engine_state.config)),
            Err(e) => eprintln!("nu-parser: {e}"),
        }
    }
}
