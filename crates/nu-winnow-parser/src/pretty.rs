//! A human-readable tree dump of the AST: [`dump`] renders a whole [`Ast`]
//! (`println!("{}", nu_winnow_parser::pretty::dump(&ast))`), and
//! [`dump_expression`] one expression.
//!
//! Each node is one line, indented two spaces per level, naming the node, its
//! details and, for most nodes, its span (`start..end`); its children follow
//! one level deeper.
//! `examples/parse.rs` prints it by default, and the tests depend on its
//! shape: the golden `<name>.ast` files of `tests/fixtures.rs` are dumps, and
//! `tests/nufmt.rs` compares dumps with the spans removed to show that
//! formatting keeps a program's structure. A change to the format means
//! regenerating the golden files (`UPDATE_FIXTURES=1`).

use std::fmt::Write;

use crate::ast::*;
use crate::span::Span;

/// Render the AST as an indented tree, one node per line, with spans,
/// followed by `Comments (n)` and `Ignored (n)` sections when they are not empty.
pub fn dump(ast: &Ast<'_>) -> String {
    let mut printer = Printer { src: ast.source, out: String::new(), depth: 0 };
    printer.block("Block", &ast.block);
    if !ast.comments.is_empty() {
        printer.line(format_args!("Comments ({})", ast.comments.len()));
        printer.depth += 1;
        for comment in &ast.comments {
            printer.line(format_args!("{} {:?}", comment.span, comment.text(ast.source)));
        }
        printer.depth -= 1;
    }
    if !ast.ignored.is_empty() {
        printer.line(format_args!("Ignored ({})", ast.ignored.len()));
        printer.depth += 1;
        for span in &ast.ignored {
            printer.line(format_args!("{span} {:?}", span.slice(ast.source)));
        }
        printer.depth -= 1;
    }
    printer.out
}

/// Render a single expression as an indented tree.
pub fn dump_expression(source: &str, expr: &Expression<'_>) -> String {
    let mut printer = Printer { src: source, out: String::new(), depth: 0 };
    printer.expr(expr);
    printer.out
}

/// Writes the dump into `out`, `depth` levels deep.
struct Printer<'a> {
    src: &'a str,
    out: String,
    depth: usize,
}

impl<'a> Printer<'a> {
    /// One line at the current depth.
    fn line(&mut self, args: std::fmt::Arguments<'_>) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
        let _ = self.out.write_fmt(args);
        self.out.push('\n');
    }

    /// The source text of `span`.
    fn text(&self, span: Span) -> &'a str {
        span.slice(self.src)
    }

    /// Run `print` one level deeper.
    fn nested(&mut self, print: impl FnOnce(&mut Self)) {
        self.depth += 1;
        print(self);
        self.depth -= 1;
    }

    /// A `{label} {span}` line for the block's contents, then its pipelines.
    /// A block that is an expression puts the expression's span in `label`, so
    /// `BlockExpr 19..24 20..23` is the expression's span (braces included),
    /// then the contents'.
    fn block(&mut self, label: &str, block: &Block<'a>) {
        self.line(format_args!("{label} {}", block.span));
        self.nested(|printer| {
            for pipeline in &block.pipelines {
                printer.pipeline(pipeline);
            }
        });
    }

    /// A keyword's block, or the variable or subexpression standing in for
    /// it, printed under `{label}Value`.
    fn block_or_value(&mut self, label: &str, block: &Block<'a>, value: Option<&Expression<'a>>) {
        match value {
            Some(value) => {
                self.line(format_args!("{label}Value"));
                self.nested(|printer| printer.expr(value));
            }
            None => self.block(label, block),
        }
    }

    /// A `Pipeline` line with its comment count and `;` terminator, then each
    /// element: its `|`, its expression and its redirection.
    fn pipeline(&mut self, pipeline: &Pipeline<'a>) {
        let comments = pipeline.leading_comments.len() + pipeline.trailing_comments.len();
        let mut extra = String::new();
        if comments > 0 {
            let _ = write!(extra, " comments={comments}");
        }
        if let Some(terminator) = pipeline.terminator {
            let _ = write!(extra, " terminator={terminator}");
        }
        self.line(format_args!("Pipeline {}{extra}", pipeline.span));
        self.nested(|printer| {
            for element in &pipeline.elements {
                if let Some(pipe) = element.pipe {
                    printer.line(format_args!("| {pipe}"));
                }
                printer.expr(&element.expr);
                if let Some(redirection) = &element.redirection {
                    printer.redirection(redirection);
                }
            }
        });
    }

    /// A `Redirect` line per target, with a file target's path nested.
    fn redirection(&mut self, r: &PipelineRedirection<'a>) {
        let target = |printer: &mut Self, t: &RedirectionTarget<'a>| match t {
            RedirectionTarget::File { op, path, .. } => {
                printer.line(format_args!("Redirect {:?} {}", op.item, op.span));
                printer.nested(|printer| printer.expr(path));
            }
            RedirectionTarget::Pipe { op } => printer.line(format_args!("Redirect {:?} {}", op.item, op.span)),
        };
        match r {
            PipelineRedirection::Single { target: t, .. } => target(self, t),
            PipelineRedirection::Separate { out, err } => {
                target(self, out);
                target(self, err);
            }
        }
    }

    /// A `Signature` line, then a `Param` line per parameter (kind, name, span,
    /// then type, completer and description) with its default nested, and an
    /// `IoType` line per input/output pair.
    fn signature(&mut self, sig: &Signature<'a>) {
        self.line(format_args!("Signature {}", sig.span));
        self.nested(|printer| {
            for param in &sig.params {
                let kind = match &param.kind {
                    ParameterKind::Required => "positional".to_string(),
                    ParameterKind::Optional => "optional".to_string(),
                    ParameterKind::Rest => "rest".to_string(),
                    ParameterKind::Flag { long, short } => format!(
                        "flag{}{}",
                        long.map(|l| format!(" --{}", l.item)).unwrap_or_default(),
                        short.map(|short| format!(" -{}", short.item)).unwrap_or_default()
                    ),
                };
                let mut extra = String::new();
                if let Some(ty) = &param.ty {
                    let _ = write!(extra, " : {}", printer.text(ty.span));
                }
                if let Some(completer) = param.completer {
                    let _ = write!(extra, " @{}", completer.item);
                }
                if !param.description.is_empty() {
                    let desc: Vec<&str> = param.description.iter().map(|comment| comment.body(printer.src)).collect();
                    let _ = write!(extra, " desc={:?}", desc.join("\n"));
                }
                printer.line(format_args!("Param {kind} `{}` {}{extra}", param.name.item, param.span));
                if let Some(default) = &param.default {
                    printer.nested(|printer| {
                        printer.line(format_args!("Default"));
                        printer.nested(|printer| printer.expr(default));
                    });
                }
            }
            for io in &sig.input_output_types {
                printer.line(format_args!(
                    "IoType {} -> {}",
                    printer.text(io.input.span),
                    printer.text(io.output.span)
                ));
            }
        });
    }

    /// Call arguments: a positional is its expression; flags, spreads and `--`
    /// get lines of their own.
    fn args(&mut self, args: &[Argument<'a>]) {
        for arg in args {
            match arg {
                Argument::Positional(expr) => self.expr(expr),
                Argument::Named(flag) => {
                    let dashes = if flag.long { "--" } else { "-" };
                    self.line(format_args!("Flag {dashes}{} {}", flag.name, flag.span));
                    if let Some(value) = &flag.value {
                        self.nested(|printer| printer.expr(value));
                    }
                }
                Argument::Spread { dots, expr } => {
                    self.line(format_args!("Spread {dots}"));
                    self.nested(|printer| printer.expr(expr));
                }
                Argument::EndOfOptions(span) => self.line(format_args!("EndOfOptions {span}")),
            }
        }
    }

    /// A `Member` line per cell-path member, with its `?` and `!`.
    fn members(&mut self, members: &[PathMember<'a>]) {
        for member in members {
            let name = match &member.kind {
                PathMemberKind::Int(int) => int.to_string(),
                PathMemberKind::String(string) => format!("{string:?}"),
            };
            let opt = if member.optional { "?" } else { "" };
            let ins = if member.case_insensitive { "!" } else { "" };
            self.line(format_args!("Member {name}{opt}{ins} {}", member.span));
        }
    }

    /// A `Pattern` line; list, record and `|` patterns nest their parts.
    fn pattern(&mut self, pat: &MatchPattern<'a>) {
        match &pat.pattern {
            Pattern::Expression(expr) => {
                self.line(format_args!("Pattern value {}", pat.span));
                self.nested(|printer| printer.expr(expr));
            }
            Pattern::Variable(variable) => self.line(format_args!("Pattern ${variable} {}", pat.span)),
            Pattern::IgnoreValue => self.line(format_args!("Pattern _ {}", pat.span)),
            Pattern::List(items) => {
                self.line(format_args!("Pattern list {}", pat.span));
                self.nested(|printer| {
                    for item in items {
                        printer.pattern(item);
                    }
                });
            }
            Pattern::Record(fields) => {
                self.line(format_args!("Pattern record {}", pat.span));
                self.nested(|printer| {
                    for (name, pattern) in fields {
                        printer.line(format_args!("Field {:?}", name.item));
                        printer.nested(|printer| printer.pattern(pattern));
                    }
                });
            }
            Pattern::Rest(name) => self.line(format_args!("Pattern rest ${} {}", name.item, pat.span)),
            // `..` is a rest pattern with no name, hence the two spaces.
            Pattern::IgnoreRest => self.line(format_args!("Pattern rest  {}", pat.span)),
            Pattern::Or(alts) => {
                self.line(format_args!("Pattern or {}", pat.span));
                self.nested(|printer| {
                    for a in alts {
                        printer.pattern(a);
                    }
                });
            }
        }
    }

    /// One expression's line, then its children one level deeper.
    fn expr(&mut self, expression: &Expression<'a>) {
        let span = expression.span;
        match &expression.expr {
            Expr::Bool(value) => self.line(format_args!("Bool {value} {span}")),
            Expr::Nothing => self.line(format_args!("Nothing {span}")),
            Expr::Int(int) => self.line(format_args!("Int {int} {span}")),
            Expr::Float(float) => self.line(format_args!("Float {float} {span}")),
            Expr::String(string) => self.line(format_args!("String {:?} {:?} {span}", string.quote, string.value)),
            Expr::StringInterpolation(interpolation) => {
                self.line(format_args!("Interpolation {:?} {span}", interpolation.quote));
                self.nested(|printer| {
                    for part in &interpolation.parts {
                        match part {
                            InterpolationPart::Text { span, value } => {
                                printer.line(format_args!("Text {value:?} {span}"))
                            }
                            InterpolationPart::Expression(expr) => printer.expr(expr),
                        }
                    }
                });
            }
            Expr::Binary(binary) => self.line(format_args!("Binary radix={} {:?} {span}", binary.radix, binary.bytes)),
            Expr::Duration(duration) => {
                self.line(format_args!("Duration {} {} {span}", duration.value, duration.unit.as_str()))
            }
            Expr::Filesize(filesize) => {
                self.line(format_args!("Filesize {} {} {span}", filesize.value, filesize.unit.as_str()))
            }
            Expr::DateTime(datetime) => self.line(format_args!("DateTime {datetime} {span}")),
            Expr::Range(range) => {
                self.line(format_args!("Range {:?} {span}", range.operator.inclusion));
                self.nested(|printer| {
                    if let Some(from) = &range.from {
                        printer.line(format_args!("From"));
                        printer.nested(|printer| printer.expr(from));
                    }
                    if let Some(next) = &range.next {
                        printer.line(format_args!("Next"));
                        printer.nested(|printer| printer.expr(next));
                    }
                    if let Some(to) = &range.to {
                        printer.line(format_args!("To"));
                        printer.nested(|printer| printer.expr(to));
                    }
                });
            }
            Expr::Var(variable) => self.line(format_args!("Var ${} {span}", variable.name)),
            Expr::CellPath(cell_path) => {
                self.line(format_args!("CellPath {span}"));
                self.nested(|printer| printer.members(&cell_path.members));
            }
            Expr::FullCellPath(full_cell_path) => {
                let implicit = if full_cell_path.implicit_head { " implicit-head" } else { "" };
                self.line(format_args!("FullCellPath{implicit} {span}"));
                self.nested(|printer| {
                    printer.expr(&full_cell_path.head);
                    printer.members(&full_cell_path.tail);
                });
            }
            Expr::List(items) => {
                self.line(format_args!("List {span}"));
                self.nested(|printer| {
                    for item in items {
                        match item {
                            ListItem::Item(expr) => printer.expr(expr),
                            ListItem::Spread { dots, expr } => {
                                printer.line(format_args!("Spread {dots}"));
                                printer.nested(|printer| printer.expr(expr));
                            }
                        }
                    }
                });
            }
            Expr::Table(table) => {
                self.line(format_args!("Table {span}"));
                self.nested(|printer| {
                    printer.line(format_args!("Columns"));
                    printer.nested(|printer| printer.expr(&table.columns));
                    for row in &table.rows {
                        printer.line(format_args!("Row"));
                        printer.nested(|printer| printer.expr(row));
                    }
                });
            }
            Expr::Record(items) => {
                self.line(format_args!("Record {span}"));
                self.nested(|printer| {
                    for item in items {
                        match item {
                            RecordItem::Pair { key, value, .. } => {
                                printer.line(format_args!("Pair"));
                                printer.nested(|printer| {
                                    printer.expr(key);
                                    printer.expr(value);
                                });
                            }
                            RecordItem::Spread { dots, expr } => {
                                printer.line(format_args!("Spread {dots}"));
                                printer.nested(|printer| printer.expr(expr));
                            }
                        }
                    }
                });
            }
            Expr::Closure(closure) => {
                self.line(format_args!("Closure {span}"));
                self.nested(|printer| {
                    if let Some(sig) = &closure.params {
                        printer.signature(sig);
                    }
                    printer.block("Body", &closure.body);
                });
            }
            Expr::Block(block) => self.block(&format!("BlockExpr {span}"), block),
            Expr::Subexpression(block) => self.block(&format!("Subexpression {span}"), block),
            Expr::BinaryOp(binary) => {
                self.line(format_args!("BinaryOp {} {span}", binary.op.item));
                self.nested(|printer| {
                    printer.expr(&binary.lhs);
                    printer.expr(&binary.rhs);
                });
            }
            Expr::UnaryNot(not) => {
                self.line(format_args!("Not {span}"));
                self.nested(|printer| printer.expr(&not.expr));
            }
            Expr::Assignment(assignment) => {
                self.line(format_args!("Assignment {} {span}", assignment.op.item.as_str()));
                self.nested(|printer| {
                    printer.expr(&assignment.lhs);
                    printer.block("Value", &assignment.rhs);
                });
            }
            Expr::Call(call) => {
                let sigil = if call.sigil.is_some() { "%" } else { "" };
                self.line(format_args!("Call `{sigil}{}` {span}", call.head.name));
                self.nested(|printer| printer.args(&call.arguments));
            }
            Expr::DynamicCall(dynamic_call) => {
                self.line(format_args!("DynamicCall {span}"));
                self.nested(|printer| {
                    printer.expr(&dynamic_call.head);
                    printer.args(&dynamic_call.arguments);
                });
            }
            Expr::ExternalCall(external_call) => {
                self.line(format_args!("ExternalCall {span}"));
                self.nested(|printer| {
                    printer.expr(&external_call.head);
                    for arg in &external_call.arguments {
                        match arg {
                            ExternalArgument::Regular(expr) => printer.expr(expr),
                            ExternalArgument::Spread { dots, expr } => {
                                printer.line(format_args!("Spread {dots}"));
                                printer.nested(|printer| printer.expr(expr));
                            }
                        }
                    }
                });
            }
            Expr::EnvShorthand(env_shorthand) => {
                self.line(format_args!("EnvShorthand {span}"));
                self.nested(|printer| {
                    for assignment in &env_shorthand.vars {
                        printer.line(format_args!("Env {} {}", assignment.name.item, assignment.span));
                        printer.nested(|printer| printer.expr(&assignment.value));
                    }
                    printer.expr(&env_shorthand.expr);
                });
            }
            Expr::AttributeBlock(attribute_block) => {
                self.line(format_args!("AttributeBlock {span}"));
                self.nested(|printer| {
                    for attr in &attribute_block.attributes {
                        printer.line(format_args!("Attribute `{}` {}", attr.name.item, attr.span));
                        printer.nested(|printer| printer.args(&attr.arguments));
                    }
                    printer.expr(&attribute_block.item);
                });
            }
            Expr::Let(binding) | Expr::Mut(binding) | Expr::Const(binding) => {
                let keyword = expression.expr.keyword().unwrap_or("let");
                let ty = binding
                    .ty
                    .as_ref()
                    .map(|annotation| format!(" : {}", self.text(annotation.span)))
                    .unwrap_or_default();
                self.line(format_args!("{keyword} {}{ty} {span}", binding.name.item));
                if let Some(value) = &binding.value {
                    self.nested(|printer| printer.block("Value", value));
                }
            }
            Expr::Def(def) => {
                let flags: Vec<_> = def.flags.iter().map(|flag| format!("{:?}", flag.item)).collect();
                let flags = if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(", ")) };
                self.line(format_args!("Def `{}`{flags} {span}", def.name.item));
                self.nested(|printer| {
                    printer.signature(&def.signature);
                    if let Some(params) = &def.body_params {
                        printer.line(format_args!("BodyParams"));
                        printer.nested(|printer| printer.signature(params));
                    }
                    printer.block("Body", &def.body);
                });
            }
            Expr::Extern(extern_declaration) => {
                self.line(format_args!("Extern `{}` {span}", extern_declaration.name.item));
                self.nested(|printer| printer.signature(&extern_declaration.signature));
            }
            Expr::Alias(alias) => {
                self.line(format_args!("Alias `{}` {span}", alias.name.item));
                if let Some(value) = &alias.value {
                    self.nested(|printer| printer.expr(value));
                }
            }
            Expr::Use(use_statement) => {
                self.line(format_args!("Use {span}"));
                self.nested(|printer| {
                    printer.expr(&use_statement.module);
                    for member in &use_statement.members {
                        match &member.kind {
                            ImportPatternMemberKind::Name(name) => {
                                printer.line(format_args!("Member {name:?} {}", member.span))
                            }
                            ImportPatternMemberKind::Glob => printer.line(format_args!("Member * {}", member.span)),
                            ImportPatternMemberKind::List(names) => {
                                let names: Vec<_> = names.iter().map(|n| n.item.as_ref()).collect();
                                printer.line(format_args!("Members {names:?} {}", member.span));
                            }
                            ImportPatternMemberKind::Ignored(ignored) => {
                                printer.line(format_args!("Member (ignored) {}", member.span));
                                printer.nested(|printer| printer.expr(ignored));
                            }
                        }
                    }
                });
            }
            Expr::Module(module) => {
                self.line(format_args!("Module {span}"));
                self.nested(|printer| {
                    printer.expr(&module.name);
                    if let Some(body) = &module.body {
                        printer.block("Body", body);
                    }
                });
            }
            Expr::Export(export) => {
                self.line(format_args!("Export {span}"));
                self.nested(|printer| printer.expr(&export.item));
            }
            Expr::ExportEnv(export_env) => self.block(&format!("ExportEnv {span}"), &export_env.body),
            Expr::If(if_expression) => {
                self.line(format_args!("If {span}"));
                self.nested(|printer| {
                    printer.line(format_args!("Condition"));
                    printer.nested(|printer| printer.expr(&if_expression.condition));
                    printer.block_or_value("Then", &if_expression.then_block, if_expression.then_value.as_deref());
                    if let Some(else_branch) = &if_expression.else_branch {
                        printer.line(format_args!("Else {}", else_branch.keyword));
                        printer.nested(|printer| printer.expr(&else_branch.body));
                    }
                });
            }
            Expr::Match(match_expression) => {
                self.line(format_args!("Match {span}"));
                self.nested(|printer| {
                    printer.expr(&match_expression.value);
                    for arm in &match_expression.arms {
                        printer.line(format_args!("Arm {}", arm.span));
                        printer.nested(|printer| {
                            printer.pattern(&arm.pattern);
                            if let Some(guard) = &arm.guard {
                                printer.line(format_args!("Guard"));
                                printer.nested(|printer| printer.expr(guard));
                            }
                            printer.expr(&arm.body);
                        });
                    }
                    if let Some(value_block) = &match_expression.value_block {
                        printer.line(format_args!("ValueBlock"));
                        printer.nested(|printer| printer.expr(value_block));
                    }
                });
            }
            Expr::For(for_loop) => {
                let ty = for_loop
                    .ty
                    .as_ref()
                    .map(|annotation| format!(" : {}", self.text(annotation.span)))
                    .unwrap_or_default();
                self.line(format_args!("For ${}{ty} {span}", for_loop.var.item));
                self.nested(|printer| {
                    printer.expr(&for_loop.iterable);
                    printer.block_or_value("Body", &for_loop.body, for_loop.body_value.as_deref());
                });
            }
            Expr::While(while_loop) => {
                self.line(format_args!("While {span}"));
                self.nested(|printer| {
                    printer.expr(&while_loop.condition);
                    printer.block_or_value("Body", &while_loop.body, while_loop.body_value.as_deref());
                });
            }
            Expr::Loop(loop_expression) => match &loop_expression.body_value {
                Some(value) => {
                    self.line(format_args!("Loop {span}"));
                    self.nested(|printer| printer.block_or_value("Body", &loop_expression.body, Some(value)));
                }
                None => self.block(&format!("Loop {span}"), &loop_expression.body),
            },
            Expr::Break => self.line(format_args!("Break {span}")),
            Expr::Continue => self.line(format_args!("Continue {span}")),
            Expr::Return(return_expression) => {
                self.line(format_args!("Return {span}"));
                if let Some(value) = &return_expression.value {
                    self.nested(|printer| printer.expr(value));
                }
            }
            Expr::Try(try_expression) => {
                self.line(format_args!("Try {span}"));
                self.nested(|printer| {
                    printer.block_or_value("Body", &try_expression.body, try_expression.body_value.as_deref());
                    for handler in &try_expression.handlers {
                        printer.line(format_args!("{:?} {}", handler.kind, handler.keyword));
                        printer.nested(|printer| printer.expr(&handler.body));
                    }
                });
            }
            Expr::Where(where_expression) => {
                self.line(format_args!("Where {span}"));
                self.nested(|printer| printer.expr(&where_expression.condition));
            }
            Expr::Garbage => self.line(format_args!("Garbage {span}")),
        }
    }
}
