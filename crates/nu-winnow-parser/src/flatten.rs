//! A flat, source-ordered view of the AST.
//!
//! [`flatten`] walks the tree and emits `(span, shape)` pairs in source order,
//! in the spirit of `nu-parser`'s `flatten_block`. This is the representation
//! `nufmt` and syntax highlighters consume: every byte of significant source
//! text is covered by exactly one shape, and gaps between shapes are
//! whitespace, comments or punctuation belonging to the enclosing construct.
//!
//! Comments are emitted as [`FlatShape::Comment`].

use crate::ast::*;
use crate::span::Span;

/// The syntactic role of a piece of source text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum FlatShape {
    /// `and`, `or`, `xor`, `not`.
    Boolean,
    /// A binary literal.
    Binary,
    /// The braces of a block.
    Block,
    /// `true` / `false`.
    Bool,
    /// The braces and pipes of a closure.
    Closure,
    /// A comment.
    Comment,
    /// A datetime literal.
    DateTime,
    /// The name of a definition (`def name`).
    Definition,
    /// An external command name.
    External,
    /// An external command argument.
    ExternalArg,
    /// A filesize literal.
    Filesize,
    /// A duration literal.
    Duration,
    /// `--flag` / `-f`.
    Flag,
    /// A float literal.
    Float,
    /// Text that failed to parse.
    Garbage,
    /// An integer literal.
    Int,
    /// An internal command name.
    InternalCall,
    /// A statement keyword (`let`, `def`, `if`, ...).
    Keyword,
    /// The brackets and commas of a list.
    List,
    /// A match pattern.
    MatchPattern,
    /// `null`.
    Nothing,
    /// A binary or assignment operator.
    Operator,
    /// `|`.
    Pipe,
    /// A range operator.
    Range,
    /// The braces of a record and the `:` of its entries.
    Record,
    /// A redirection operator.
    Redirection,
    /// A signature and its parameters.
    Signature,
    /// A string literal (any quoting).
    String,
    /// The literal parts of an interpolated string.
    StringInterpolation,
    /// The brackets of a table.
    Table,
    /// A type annotation.
    Type,
    /// A variable reference.
    Variable,
    /// A variable declaration (`let x`, closure parameters).
    VarDecl,
    /// A cell-path member.
    CellPath,
    /// `@attribute`.
    Attribute,
    /// Text nu-parser accepts and discards (see [`Ast::ignored`]).
    Ignored,
}

/// Flatten an AST into source-ordered `(span, shape)` pairs.
pub fn flatten(ast: &Ast<'_>) -> Vec<(Span, FlatShape)> {
    let mut flattener = Flattener { src: ast.source, out: Vec::new() };
    flattener.visit_block(&ast.block);
    // Comments and ignored text win over the shapes of the constructs they
    // sit in (the gaps of a list or record), so cut them out of every other shape.
    let mut cuts: Vec<(Span, FlatShape)> =
        ast.comments.iter().map(|comment| (comment.span, FlatShape::Comment)).collect();
    cuts.extend(ast.ignored.iter().map(|ignored| (*ignored, FlatShape::Ignored)));
    cuts.sort_by_key(|(cut, _)| (cut.start, cut.end));
    let mut out = Vec::with_capacity(flattener.out.len() + cuts.len());
    for (span, shape) in flattener.out {
        let mut start = span.start;
        for (cut, _) in cuts.iter().filter(|(cut, _)| cut.start < span.end && span.start < cut.end) {
            if cut.start > start {
                out.push((Span::new(start, cut.start), shape));
            }
            start = start.max(cut.end);
        }
        if start < span.end {
            out.push((Span::new(start, span.end), shape));
        }
    }
    out.extend(cuts);
    out.sort_by_key(|(span, _)| (span.start, span.end));
    out.dedup();
    out
}

struct Flattener<'s> {
    src: &'s str,
    out: Vec<(Span, FlatShape)>,
}

impl Flattener<'_> {
    fn push(&mut self, span: Span, shape: FlatShape) {
        if !span.is_empty() {
            self.out.push((span, shape));
        }
    }

    /// The declaration of a variable: the name plus a leading `$` and a
    /// trailing `:` when they are written (`let $x: int`, `for x: int in`).
    fn var_decl(&mut self, name: Span) {
        let start = if self.src[..name.start].ends_with('$') { name.start - 1 } else { name.start };
        let end = if self.src[name.end..].starts_with(':') { name.end + 1 } else { name.end };
        self.push(Span::new(start, end), FlatShape::VarDecl);
    }

    /// Emit `shape` for the parts of `outer` not covered by `inner` spans
    /// (the delimiters and punctuation of a collection).
    fn gaps(&mut self, outer: Span, inner: impl Iterator<Item = Span>, shape: FlatShape) {
        let mut cursor = outer.start;
        for covered in inner {
            if covered.start > cursor {
                self.push(Span::new(cursor, covered.start), shape);
            }
            cursor = cursor.max(covered.end);
        }
        if outer.end > cursor {
            self.push(Span::new(cursor, outer.end), shape);
        }
    }

    fn args(&mut self, args: &[Argument<'_>]) {
        for arg in args {
            match arg {
                Argument::Positional(expr) => self.visit_expression(expr),
                Argument::Named(flag) => match &flag.value {
                    Some(v) => {
                        self.push(Span::new(flag.span.start, v.span.start), FlatShape::Flag);
                        self.visit_expression(v);
                    }
                    None => self.push(flag.span, FlatShape::Flag),
                },
                Argument::Spread { dots, expr } => {
                    self.push(*dots, FlatShape::Operator);
                    self.visit_expression(expr);
                }
                Argument::EndOfOptions(span) => self.push(*span, FlatShape::Flag),
            }
        }
    }

    fn members(&mut self, members: &[PathMember<'_>]) {
        for member in members {
            self.push(member.span, FlatShape::CellPath);
        }
    }

    fn block_braces(&mut self, expr_span: Span, block: &Block<'_>, shape: FlatShape) {
        self.push(Span::new(expr_span.start, block.span.start), shape);
        self.visit_block(block);
        self.push(Span::new(block.span.end, expr_span.end), shape);
    }

    /// A keyword's block, or the variable or subexpression standing in for it.
    fn block_or_value(&mut self, expr_span: Span, block: &Block<'_>, value: Option<&Expression<'_>>) {
        match value {
            Some(value) => self.visit_expression(value),
            None => self.block_braces(expr_span, block, FlatShape::Block),
        }
    }
}

impl<'a> Visitor<'a> for Flattener<'_> {
    fn visit_pipeline_element(&mut self, element: &PipelineElement<'a>) {
        // After `e>|` the "pipe" is the redirection operator, already emitted.
        if let Some(pipe) = element.pipe
            && &self.src[pipe.range()] == "|"
        {
            self.push(pipe, FlatShape::Pipe);
        }
        walk_pipeline_element(self, element);
    }

    fn visit_redirection(&mut self, redirection: &PipelineRedirection<'a>) {
        let mut target = |t: &RedirectionTarget<'a>| {
            self.push(t.op_span(), FlatShape::Redirection);
            if let RedirectionTarget::File { path, .. } = t {
                self.visit_expression(path);
            }
        };
        match redirection {
            PipelineRedirection::Single { target: t, .. } => target(t),
            PipelineRedirection::Separate { out, err } => {
                target(out);
                target(err);
            }
        }
    }

    fn visit_signature(&mut self, signature: &Signature<'a>) {
        for parameter in &signature.params {
            match &parameter.kind {
                ParameterKind::Flag { .. } => self.push(parameter.name.span, FlatShape::Flag),
                _ => self.push(parameter.name.span, FlatShape::VarDecl),
            }
            if let Some(ty) = &parameter.ty {
                self.push(ty.span, FlatShape::Type);
            }
            if let Some(completer) = parameter.completer {
                self.push(completer.span, FlatShape::InternalCall);
            }
            if let Some(default) = &parameter.default {
                self.visit_expression(default);
            }
        }
        for io in &signature.input_output_types {
            self.push(io.input.span, FlatShape::Type);
            self.push(io.arrow, FlatShape::Operator);
            self.push(io.output.span, FlatShape::Type);
        }
        let covered: Vec<Span> = self
            .out
            .iter()
            .filter(|(span, _)| signature.span.start <= span.start && span.end <= signature.span.end)
            .map(|(span, _)| *span)
            .collect();
        let mut covered = covered;
        covered.sort_by_key(|span| span.start);
        self.gaps(signature.span, covered.into_iter(), FlatShape::Signature);
    }

    fn visit_match_pattern(&mut self, pattern: &MatchPattern<'a>) {
        match &pattern.pattern {
            Pattern::Expression(expr) => self.visit_expression(expr),
            _ => self.push(pattern.span, FlatShape::MatchPattern),
        }
    }

    fn visit_expression(&mut self, expression: &Expression<'a>) {
        let span = expression.span;
        // Keyword statements (`let`, `if`, ...) start with their keyword.
        let keyword = expression.keyword_span().unwrap_or(Span::point(span.start));
        if !keyword.is_empty() {
            self.push(keyword, FlatShape::Keyword);
        }
        match &expression.expr {
            Expr::Bool(_) => self.push(span, FlatShape::Bool),
            Expr::Nothing => self.push(span, FlatShape::Nothing),
            Expr::Int(_) => self.push(span, FlatShape::Int),
            Expr::Float(_) => self.push(span, FlatShape::Float),
            Expr::String(_) => self.push(span, FlatShape::String),
            Expr::StringInterpolation(interpolation) => {
                let mut inner = Vec::new();
                for part in &interpolation.parts {
                    match part {
                        InterpolationPart::Text { span, .. } => {
                            self.push(*span, FlatShape::StringInterpolation);
                            inner.push(*span);
                        }
                        InterpolationPart::Expression(expr) => {
                            self.visit_expression(expr);
                            inner.push(expr.span);
                        }
                    }
                }
                self.gaps(span, inner.into_iter(), FlatShape::StringInterpolation);
            }
            Expr::Binary(_) => self.push(span, FlatShape::Binary),
            Expr::Duration(_) => self.push(span, FlatShape::Duration),
            Expr::Filesize(_) => self.push(span, FlatShape::Filesize),
            Expr::DateTime(_) => self.push(span, FlatShape::DateTime),
            Expr::Range(range) => {
                if let Some(from) = &range.from {
                    self.visit_expression(from);
                }
                if let Some(next_op_span) = range.operator.next_op_span {
                    self.push(next_op_span, FlatShape::Range);
                }
                if let Some(next) = &range.next {
                    self.visit_expression(next);
                }
                self.push(range.operator.span, FlatShape::Range);
                if let Some(to) = &range.to {
                    self.visit_expression(to);
                }
            }
            Expr::Var(_) => self.push(span, FlatShape::Variable),
            Expr::CellPath(cell_path) => {
                self.push(Span::new(span.start, span.start + 2), FlatShape::CellPath);
                self.members(&cell_path.members);
            }
            Expr::FullCellPath(full_cell_path) => {
                self.visit_expression(&full_cell_path.head);
                self.members(&full_cell_path.tail);
            }
            Expr::List(items) => {
                let spans: Vec<Span> = items.iter().map(ListItem::span).collect();
                for item in items {
                    match item {
                        ListItem::Item(expr) => self.visit_expression(expr),
                        ListItem::Spread { dots, expr } => {
                            self.push(*dots, FlatShape::Operator);
                            self.visit_expression(expr);
                        }
                    }
                }
                self.gaps(span, spans.into_iter(), FlatShape::List);
            }
            Expr::Table(table) => {
                self.visit_expression(&table.columns);
                for row in &table.rows {
                    self.visit_expression(row);
                }
                let spans = std::iter::once(table.columns.span).chain(table.rows.iter().map(|r| r.span));
                self.gaps(span, spans, FlatShape::Table);
            }
            Expr::Record(items) => {
                let mut spans = Vec::new();
                for item in items {
                    match item {
                        RecordItem::Pair { key, colon, value } => {
                            self.visit_expression(key);
                            self.push(*colon, FlatShape::Record);
                            self.visit_expression(value);
                            spans.push(key.span);
                            spans.push(*colon);
                            spans.push(value.span);
                        }
                        RecordItem::Spread { dots, expr } => {
                            self.push(*dots, FlatShape::Operator);
                            self.visit_expression(expr);
                            spans.push(*dots);
                            spans.push(expr.span);
                        }
                    }
                }
                self.gaps(span, spans.into_iter(), FlatShape::Record);
            }
            Expr::Closure(closure) => {
                let body_start = closure.params.as_ref().map_or(closure.body.span.start, |params| params.span.start);
                self.push(Span::new(span.start, body_start), FlatShape::Closure);
                if let Some(sig) = &closure.params {
                    self.visit_signature(sig);
                }
                self.visit_block(&closure.body);
                self.push(Span::new(closure.body.span.end, span.end), FlatShape::Closure);
            }
            Expr::Block(block) => self.block_braces(span, block, FlatShape::Block),
            Expr::Subexpression(block) => self.block_braces(span, block, FlatShape::Block),
            Expr::BinaryOp(binary) => {
                self.visit_expression(&binary.lhs);
                let shape = if matches!(binary.op.item, Operator::Boolean(_)) {
                    FlatShape::Boolean
                } else {
                    FlatShape::Operator
                };
                self.push(binary.op.span, shape);
                self.visit_expression(&binary.rhs);
            }
            Expr::UnaryNot(not) => {
                self.push(not.not_span, FlatShape::Boolean);
                self.visit_expression(&not.expr);
            }
            Expr::Assignment(assignment) => {
                self.visit_expression(&assignment.lhs);
                self.push(assignment.op.span, FlatShape::Operator);
                self.visit_block(&assignment.rhs);
            }
            Expr::Call(call) => {
                match call.sigil {
                    Some(sigil) if sigil.end == call.head.span.start => {
                        self.push(sigil.merge(call.head.span), FlatShape::InternalCall);
                    }
                    Some(sigil) => {
                        self.push(sigil, FlatShape::InternalCall);
                        self.push(call.head.span, FlatShape::InternalCall);
                    }
                    None => self.push(call.head.span, FlatShape::InternalCall),
                }
                self.args(&call.arguments);
            }
            Expr::DynamicCall(dynamic_call) => {
                self.push(dynamic_call.sigil, FlatShape::InternalCall);
                self.visit_expression(&dynamic_call.head);
                self.args(&dynamic_call.arguments);
            }
            Expr::ExternalCall(external_call) => {
                if let Some(caret) = external_call.caret {
                    self.push(caret, FlatShape::External);
                }
                match &external_call.head.expr {
                    Expr::String(_) => self.push(external_call.head.span, FlatShape::External),
                    _ => self.visit_expression(&external_call.head),
                }
                for arg in &external_call.arguments {
                    match arg {
                        ExternalArgument::Regular(expr) => match &expr.expr {
                            Expr::String(_) => self.push(expr.span, FlatShape::ExternalArg),
                            _ => self.visit_expression(expr),
                        },
                        ExternalArgument::Spread { dots, expr } => {
                            self.push(*dots, FlatShape::Operator);
                            self.visit_expression(expr);
                        }
                    }
                }
            }
            Expr::EnvShorthand(env_shorthand) => {
                for assignment in &env_shorthand.vars {
                    self.push(Span::new(assignment.span.start, assignment.value.span.start), FlatShape::VarDecl);
                    self.visit_expression(&assignment.value);
                }
                self.visit_expression(&env_shorthand.expr);
            }
            Expr::AttributeBlock(attribute_block) => {
                for attr in &attribute_block.attributes {
                    self.push(Span::new(attr.span.start, attr.name.span.end), FlatShape::Attribute);
                    self.args(&attr.arguments);
                }
                self.visit_expression(&attribute_block.item);
            }
            Expr::Let(binding) | Expr::Mut(binding) | Expr::Const(binding) => {
                self.var_decl(binding.name.span);
                if let Some(ty) = &binding.ty {
                    self.push(ty.span, FlatShape::Type);
                }
                if let Some(eq) = binding.eq {
                    self.push(eq, FlatShape::Operator);
                }
                if let Some(value) = &binding.value {
                    self.visit_block(value);
                }
            }
            Expr::Def(def) => {
                for flag in &def.flags {
                    self.push(flag.span, FlatShape::Flag);
                }
                self.push(def.name.span, FlatShape::Definition);
                self.visit_signature(&def.signature);
                match &def.body_params {
                    // `def f [] {|x| }`: like a closure, the braces around the parameters.
                    Some(body_params) => {
                        self.push(Span::new(def.signature.span.end, body_params.span.start), FlatShape::Block);
                        self.visit_signature(body_params);
                        self.visit_block(&def.body);
                        self.push(Span::new(def.body.span.end, span.end), FlatShape::Block);
                    }
                    None => self.block_braces(Span::new(def.signature.span.end, span.end), &def.body, FlatShape::Block),
                }
            }
            Expr::Extern(extern_declaration) => {
                self.push(extern_declaration.name.span, FlatShape::Definition);
                self.visit_signature(&extern_declaration.signature);
            }
            Expr::Alias(alias) => {
                self.push(alias.name.span, FlatShape::Definition);
                self.push(alias.eq, FlatShape::Operator);
                if let Some(value) = &alias.value {
                    self.visit_expression(value);
                }
            }
            Expr::Use(use_statement) => {
                self.visit_expression(&use_statement.module);
                for member in &use_statement.members {
                    match &member.kind {
                        ImportPatternMemberKind::List(names) => {
                            for name in names {
                                self.push(name.span, FlatShape::String);
                            }
                            self.gaps(member.span, names.iter().map(|n| n.span), FlatShape::List);
                        }
                        ImportPatternMemberKind::Ignored(ignored) => {
                            self.visit_expression(ignored);
                            self.gaps(member.span, std::iter::once(ignored.span), FlatShape::Ignored);
                        }
                        _ => self.push(member.span, FlatShape::String),
                    }
                }
            }
            Expr::Module(module) => {
                self.visit_expression(&module.name);
                if let Some(body) = &module.body {
                    self.block_braces(Span::new(module.name.span.end, span.end), body, FlatShape::Block);
                }
            }
            Expr::Export(export) => {
                self.visit_expression(&export.item);
            }
            Expr::ExportEnv(export_env) => {
                self.block_braces(Span::new(keyword.end, span.end), &export_env.body, FlatShape::Block);
            }
            Expr::If(if_expression) => {
                self.visit_expression(&if_expression.condition);
                let then_end = if_expression.else_branch.as_ref().map_or(span.end, |e| e.keyword.start);
                self.block_or_value(
                    Span::new(if_expression.condition.span.end, then_end),
                    &if_expression.then_block,
                    if_expression.then_value.as_deref(),
                );
                if let Some(else_branch) = &if_expression.else_branch {
                    self.push(else_branch.keyword, FlatShape::Keyword);
                    self.visit_expression(&else_branch.body);
                }
            }
            Expr::Match(match_expression) => {
                self.visit_expression(&match_expression.value);
                let mut inner = Vec::new();
                for arm in &match_expression.arms {
                    self.visit_match_pattern(&arm.pattern);
                    inner.push(arm.pattern.span);
                    if let Some(guard) = &arm.guard {
                        self.visit_expression(guard);
                        inner.push(guard.span);
                    }
                    self.push(arm.arrow, FlatShape::Operator);
                    inner.push(arm.arrow);
                    self.visit_expression(&arm.body);
                    inner.push(arm.body.span);
                }
                match &match_expression.value_block {
                    Some(b) => self.visit_expression(b),
                    None => self.gaps(match_expression.block_span, inner.into_iter(), FlatShape::Block),
                }
            }
            Expr::For(for_loop) => {
                self.var_decl(for_loop.var.span);
                if let Some(ty) = &for_loop.ty {
                    self.push(ty.span, FlatShape::Type);
                }
                self.push(for_loop.in_keyword, FlatShape::Keyword);
                self.visit_expression(&for_loop.iterable);
                self.block_or_value(
                    Span::new(for_loop.iterable.span.end, span.end),
                    &for_loop.body,
                    for_loop.body_value.as_deref(),
                );
            }
            Expr::While(while_loop) => {
                self.visit_expression(&while_loop.condition);
                self.block_or_value(
                    Span::new(while_loop.condition.span.end, span.end),
                    &while_loop.body,
                    while_loop.body_value.as_deref(),
                );
            }
            Expr::Loop(loop_expression) => {
                self.block_or_value(
                    Span::new(keyword.end, span.end),
                    &loop_expression.body,
                    loop_expression.body_value.as_deref(),
                );
            }
            Expr::Break | Expr::Continue => {}
            Expr::Return(return_expression) => {
                if let Some(value) = &return_expression.value {
                    self.visit_expression(value);
                }
            }
            Expr::Try(try_expression) => {
                let body_end = try_expression.handlers.first().map_or(span.end, |h| h.keyword.start);
                self.block_or_value(
                    Span::new(keyword.end, body_end),
                    &try_expression.body,
                    try_expression.body_value.as_deref(),
                );
                for handler in &try_expression.handlers {
                    self.push(handler.keyword, FlatShape::Keyword);
                    self.visit_expression(&handler.body);
                }
            }
            Expr::Where(where_expression) => {
                self.visit_expression(&where_expression.condition);
            }
            Expr::Garbage => self.push(span, FlatShape::Garbage),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cover_source_in_order_without_overlap() {
        let src = "let x = [1 2] | each {|i| $i + 1 } # c\ndef f [a: int] { $a }";
        let ast = crate::parse(src).unwrap();
        let shapes = flatten(&ast);
        let mut last_end = 0;
        for (span, shape) in &shapes {
            assert!(span.start >= last_end, "overlap at {span} ({shape:?})");
            last_end = span.end;
        }
        let covered: usize = shapes.iter().map(|(s, _)| s.len()).sum();
        let significant = src.bytes().filter(|b| !b.is_ascii_whitespace()).count();
        assert!(covered >= significant, "covered {covered} of {significant} significant bytes");
        assert!(shapes.iter().any(|(_, s)| *s == FlatShape::Keyword));
        assert!(shapes.iter().any(|(_, s)| *s == FlatShape::Comment));
        assert!(shapes.iter().any(|(_, s)| *s == FlatShape::Closure));
    }
}
