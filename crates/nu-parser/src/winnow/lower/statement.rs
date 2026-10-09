//! Blocks, statements and pipelines (the classic parser's `parse_block`,
//! `parse_builtin_commands`, `parse_pipeline` and `parse_pipeline_element`), and the hand-over
//! of a statement to the classic parser.

use nu_protocol::{
    Module, ParseError, Span, SyntaxShape, Type,
    ast::{
        Block, Expr, Pipeline, PipelineElement, PipelineRedirection, RedirectionSource,
        RedirectionTarget,
    },
    engine::StateWorkingSet,
};
use nu_winnow_parser::lex::RedirectionSource as WinnowSource;
use nu_winnow_parser::{Span as WSpan, ast as w};

use super::{Lower, Lowered, Unlowered};
use crate::{
    lex::lex,
    lite_parser::lite_parse,
    parse_captures_compile::wrap_element_with_collect,
    parse_module::parse_module_pipeline,
    parse_pipelines::{finish_block, parse_pipeline},
    winnow::{
        driver::{BlockKind, parse_block},
        stats,
    },
};

/// Where a statement goes: an ordinary block, or a module's body.
pub(in crate::winnow) enum StatementTarget<'t> {
    /// A statement of an ordinary block; `input_type` is the block's input, which only its
    /// first statement receives.
    Block { input_type: Option<&'t Type> },
    /// A statement of the body of the module `name`, covering `span`; exports add to `module`.
    Module {
        name: &'t [u8],
        module: &'t mut Module,
        span: Span,
    },
}

impl<'s> Lower<'_, '_, 's> {
    /// A statement lowered, or parsed by the classic parser from `span` when the lowering gives
    /// it back, added to `out`. Whatever the lowering reported or registered before giving up is
    /// dropped.
    pub(in crate::winnow) fn statement_or_classic(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        span: Span,
        target: &mut StatementTarget<'_>,
        out: &mut Vec<Pipeline>,
    ) {
        let errors = self.working_set.parse_errors.len();
        let warnings = self.working_set.parse_warnings.len();
        let compile_errors = self.working_set.compile_errors.len();
        match self.statement(pipeline, target) {
            Ok(pipeline) => {
                stats::record_lowered_statement();
                out.push(pipeline);
            }
            Err(reason) => {
                self.working_set.parse_errors.truncate(errors);
                self.working_set.parse_warnings.truncate(warnings);
                self.working_set.compile_errors.truncate(compile_errors);
                stats::record_unlowered(span, &reason, self.working_set);
                parse_classic(self.working_set, span, target, true, out);
            }
        }
    }

    /// One statement (`parse_pipeline`, or `parse_module_pipeline` in a module's body).
    fn statement(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        target: &mut StatementTarget<'_>,
    ) -> Lowered<Pipeline> {
        match target {
            StatementTarget::Block { input_type } => self.pipeline(pipeline, *input_type),
            StatementTarget::Module { name, module, .. } => {
                self.module_statement(pipeline, name, module)
            }
        }
    }

    /// A statement of a module's body (`parse_module_pipeline`): definitions, constants and
    /// `export-env` blocks, and their exports added to `module`.
    fn module_statement(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        module_name: &[u8],
        module: &mut Module,
    ) -> Lowered<Pipeline> {
        let [element] = pipeline.elements.as_slice() else {
            return Err(Unlowered::Error);
        };
        let (command, attributes) = match &element.expr.expr {
            w::Expr::AttributeBlock(block) => (&*block.item, block.attributes.as_slice()),
            _ => (&element.expr, &[][..]),
        };
        let expression = match &command.expr {
            w::Expr::Def(def) => self.def(pipeline, command, def, attributes, None)?.0,
            w::Expr::Extern(extern_def) => {
                self.extern_def(pipeline, command, extern_def, attributes, None)?
            }
            w::Expr::Const(binding) if attributes.is_empty() => {
                self.const_statement(command, binding)?.0
            }
            w::Expr::ExportEnv(export_env) if attributes.is_empty() => {
                let (expression, block_id) = self.export_env(command, export_env)?;
                module.add_env_block(block_id);
                expression
            }
            w::Expr::Export(export) => match &export.item.expr {
                w::Expr::Def(def) => {
                    let (expression, decl) =
                        self.def(pipeline, command, def, attributes, Some(module_name))?;
                    if let Some((name, decl_id)) = decl {
                        self.add_export_decl(module, module_name, name, decl_id, &expression);
                    }
                    expression
                }
                w::Expr::Extern(extern_def) => {
                    let expression = self.extern_def(
                        pipeline,
                        command,
                        extern_def,
                        attributes,
                        Some(module_name),
                    )?;
                    let name = extern_def.name.item.as_bytes();
                    match self.working_set.find_decl(name) {
                        Some(decl_id) => self.add_export_decl(
                            module,
                            module_name,
                            name.to_vec(),
                            decl_id,
                            &expression,
                        ),
                        None => return Err(Unlowered::Error),
                    }
                    expression
                }
                w::Expr::Const(binding) if attributes.is_empty() => {
                    let (expression, var_span) = self.export_const(command, export, binding)?;
                    let name = self.working_set.get_span_contents(var_span).to_vec();
                    let var_id = self
                        .working_set
                        .find_variable(&name)
                        .ok_or(Unlowered::Error)?;
                    self.working_set
                        .get_constant(var_id)
                        .map_err(|_| Unlowered::Error)?;
                    module.add_variable(name, var_id);
                    expression
                }
                _ => return Err(Unlowered::Unsupported("export in a module")),
            },
            _ => return Err(Unlowered::Unsupported("module statement")),
        };
        Ok(Pipeline::from_vec(vec![expression]))
    }

    /// Add an exported command to a module: as its `main`, or under its name.
    fn add_export_decl(
        &mut self,
        module: &mut Module,
        module_name: &[u8],
        name: Vec<u8>,
        decl_id: nu_protocol::DeclId,
        expression: &nu_protocol::ast::Expression,
    ) {
        if name != b"main" {
            module.add_decl(name, decl_id);
        } else if module.main.is_some() {
            let span = match &expression.expr {
                Expr::Call(call) => call.head,
                _ => expression.span,
            };
            self.working_set.error(ParseError::ModuleDoubleMain(
                String::from_utf8_lossy(module_name).to_string(),
                span,
            ));
        } else {
            module.main = Some(decl_id);
        }
    }

    /// `export const` (in a script or a module): the constant, made a call to `export const`
    /// covering the statement. Returns it with the variable's name span.
    fn export_const(
        &mut self,
        command: &w::Expression<'s>,
        export: &w::Export<'s>,
        binding: &w::Binding<'s>,
    ) -> Lowered<(nu_protocol::ast::Expression, Span)> {
        let (mut expression, var_span) = self.const_statement(&export.item, binding)?;
        let span = self.span(command.span);
        self.export_call(&mut expression, "export const", span)?;
        if let Expr::Call(call) = &mut expression.expr {
            call.head = Span::new(span.start, call.head.end);
        }
        Ok((expression, var_span))
    }

    /// A pipeline of an ordinary block (`parse_pipeline`): the output type of each element is
    /// the input type of the next, and an element after the first that uses `$in` is wrapped
    /// in a `collect`.
    fn pipeline(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        match pipeline.elements.as_slice() {
            [] => Err(Unlowered::Unsupported("empty pipeline")),
            [single] => self.builtin_commands(pipeline, single, input_type),
            [first, rest @ ..] => {
                let pipes = self.classic_pipes(pipeline);
                let mut current = input_type.cloned().unwrap_or(Type::Any);
                let mut elements = Vec::with_capacity(pipeline.elements.len());
                let element = self.pipeline_element(first, &current, pipes[0])?;
                current = element.expr.ty.clone();
                elements.push(element);
                for (index, element) in rest.iter().enumerate() {
                    let input = current.clone();
                    let element = self.pipeline_element(element, &current, pipes[index + 1])?;
                    current = element.expr.ty.clone();
                    let element = if element.has_in_variable(self.working_set) {
                        wrap_element_with_collect(self.working_set, element, Some(&input))
                    } else {
                        element
                    };
                    elements.push(element);
                }
                Ok(Pipeline { elements })
            }
        }
    }

    /// A pipeline of one command, which may be a statement (`parse_builtin_commands`).
    fn builtin_commands(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        element: &w::PipelineElement<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        let expr = match &element.expr.expr {
            w::Expr::Def(def) => self.def(pipeline, &element.expr, def, &[], None)?.0,
            w::Expr::Extern(extern_def) => {
                self.extern_def(pipeline, &element.expr, extern_def, &[], None)?
            }
            w::Expr::Const(binding) => self.const_statement(&element.expr, binding)?.0,
            w::Expr::ExportEnv(export_env) => self.export_env(&element.expr, export_env)?.0,
            // `export def`, `export extern` and `export const` outside a module
            // (`parse_export_in_block`).
            w::Expr::Export(export) => match &export.item.expr {
                w::Expr::Def(def) => self.def(pipeline, &element.expr, def, &[], None)?.0,
                w::Expr::Extern(extern_def) => {
                    self.extern_def(pipeline, &element.expr, extern_def, &[], None)?
                }
                w::Expr::Const(binding) => self.export_const(&element.expr, export, binding)?.0,
                _ => return Err(Unlowered::Unsupported("export")),
            },
            w::Expr::AttributeBlock(block) => {
                let command = &*block.item;
                let attributes = block.attributes.as_slice();
                let definition = match &command.expr {
                    w::Expr::Export(export) => &export.item.expr,
                    other => other,
                };
                match definition {
                    w::Expr::Def(def) => self.def(pipeline, command, def, attributes, None)?.0,
                    w::Expr::Extern(extern_def) => {
                        self.extern_def(pipeline, command, extern_def, attributes, None)?
                    }
                    _ => return Err(Unlowered::Unsupported("attributes")),
                }
            }
            w::Expr::Let(binding) => return self.let_statement(element, binding, input_type),
            w::Expr::Mut(binding) => return self.mut_statement(element, binding),
            w::Expr::For(for_loop) => self.for_statement(element, for_loop)?,
            w::Expr::Alias(_) => return Err(Unlowered::Unsupported("alias")),
            w::Expr::Module(_) => return Err(Unlowered::Unsupported("module")),
            w::Expr::Use(_) => return Err(Unlowered::Unsupported("use")),
            _ => {
                let pipe = self.classic_pipes(pipeline)[0];
                let element =
                    self.pipeline_element(element, input_type.unwrap_or(&Type::Any), pipe)?;
                // An alias of `overlay use` and friends has side effects the lowering does not
                // apply.
                if let Expr::Call(call) = &element.expr.expr
                    && matches!(
                        self.working_set.get_decl(call.decl_id).name(),
                        "overlay hide" | "overlay new" | "overlay use"
                    )
                {
                    return Err(Unlowered::Unsupported("overlay"));
                }
                return Ok(Pipeline {
                    elements: vec![element],
                });
            }
        };
        Ok(Pipeline::from_vec(vec![expr]))
    }

    /// The `pipe` of each element as the classic lite parser sets it: the first `|` after an
    /// element, except for the last element, which keeps a `|` after it that no command follows
    /// (`ls |` at the end of a block), or else the last of two or more `|` before it (and the
    /// first element a leading `|`), which empty commands between them hand on. The winnow
    /// parser keeps, with each element, the last `|` before it.
    fn classic_pipes(&self, pipeline: &w::Pipeline<'s>) -> Vec<Option<WSpan>> {
        let elements = &pipeline.elements;
        let last = elements.len() - 1;
        (0..elements.len())
            .map(|index| {
                if index < last {
                    let next = &elements[index + 1];
                    let gap_end = next.pipe.map_or(next.span.start, |pipe| pipe.start);
                    self.first_pipe(elements[index].span.end, gap_end)
                        .or(next.pipe)
                } else if let Some(dangling) = self.dangling_pipe(&elements[index]) {
                    Some(dangling)
                } else {
                    let pipe = elements[index].pipe?;
                    let gap_start = match index {
                        0 => return Some(pipe),
                        _ => elements[index - 1].span.end,
                    };
                    self.first_pipe(gap_start, pipe.start).map(|_| pipe)
                }
            })
            .collect()
    }

    /// A `|` (or `e>|`) right after the last element of a pipeline, which no command follows:
    /// the winnow parser drops it, the classic lite parser keeps it with the element.
    fn dangling_pipe(&self, element: &w::PipelineElement<'s>) -> Option<WSpan> {
        if let Some(w::PipelineRedirection::Single {
            target: w::RedirectionTarget::Pipe { op },
            ..
        }) = &element.redirection
        {
            return Some(op.span);
        }
        let after = self.source.get(element.span.end..)?;
        let start = element.span.end + (after.len() - after.trim_start_matches([' ', '\t']).len());
        (self.source[start..].starts_with('|') && !self.source[start..].starts_with("||"))
            .then(|| WSpan::new(start, start + 1))
    }

    /// The first `|` between `start` and `end` of the source, outside comments.
    fn first_pipe(&self, start: usize, end: usize) -> Option<WSpan> {
        let text = self.source.get(start..end)?;
        let mut in_comment = false;
        for (offset, byte) in text.bytes().enumerate() {
            match byte {
                b'\n' => in_comment = false,
                b'#' => in_comment = true,
                b'|' if !in_comment => {
                    return Some(WSpan::new(start + offset, start + offset + 1));
                }
                _ => {}
            }
        }
        None
    }

    /// One command of a pipeline with its redirection (`parse_pipeline_element`); `pipe` is
    /// the `|` the classic parser keeps with it.
    fn pipeline_element(
        &mut self,
        element: &w::PipelineElement<'s>,
        input_type: &Type,
        pipe: Option<WSpan>,
    ) -> Lowered<PipelineElement> {
        let expr = self.expression(&element.expr, Some(input_type))?;
        let redirection = match &element.redirection {
            Some(redirection) => Some(self.redirection(redirection)?),
            None => None,
        };
        Ok(PipelineElement {
            pipe: pipe.map(|pipe| self.span(pipe)),
            expr,
            redirection,
        })
    }

    /// The redirection of a command (`parse_redirection`).
    fn redirection(
        &mut self,
        redirection: &w::PipelineRedirection<'s>,
    ) -> Lowered<PipelineRedirection> {
        Ok(match redirection {
            w::PipelineRedirection::Single { source, target } => PipelineRedirection::Single {
                source: redirection_source(*source),
                target: self.redirection_target(target)?,
            },
            w::PipelineRedirection::Separate { out, err } => PipelineRedirection::Separate {
                out: self.redirection_target(out)?,
                err: self.redirection_target(err)?,
            },
        })
    }

    fn redirection_target(
        &mut self,
        target: &w::RedirectionTarget<'s>,
    ) -> Lowered<RedirectionTarget> {
        Ok(match target {
            w::RedirectionTarget::File { op, append, path } => RedirectionTarget::File {
                expr: self.value(path, &SyntaxShape::Any, None)?,
                append: *append,
                span: self.span(op.span),
            },
            w::RedirectionTarget::Pipe { op } => RedirectionTarget::Pipe {
                span: self.span(op.span),
            },
        })
    }

    /// A block from the winnow tree (`parse_block`), covering `span`: a new scope when
    /// `scoped`, its definitions declared first, then its statements; statements the lowering
    /// gives back are parsed by the classic parser.
    pub(super) fn block(
        &mut self,
        block: &w::Block<'s>,
        span: Span,
        scoped: bool,
        is_subexpression: bool,
        input_type: Option<&Type>,
    ) -> Lowered<Block> {
        // A statement that changes which commands exist (`use`, `overlay use`, `source`, ...)
        // changes how the statements after it parse, which the winnow parser could not know
        // when it parsed this block as part of its statement: the block is parsed again, one
        // statement at a time, as a file is.
        if block.pipelines.iter().any(changes_commands) {
            let inner = self.span(block.span);
            let kind = BlockKind::Block {
                scoped,
                is_subexpression,
                input_type: input_type.cloned(),
            };
            return parse_block(
                self.working_set,
                self.source,
                self.offset,
                span,
                inner,
                kind,
            )
            .map(|(block, _)| block)
            .ok_or(Unlowered::Unsupported("block that changes commands"));
        }
        if scoped {
            self.working_set.enter_scope();
        }
        // A definition the lowering cannot predeclare leaves the whole block to the classic
        // parser, which predeclares it.
        for pipeline in &block.pipelines {
            if let [element] = pipeline.elements.as_slice()
                && let Err(unlowered) = self.predecl_statement(&element.expr)
            {
                if scoped {
                    self.working_set.exit_scope();
                }
                return Err(unlowered);
            }
        }
        let mut out = Block::new_with_capacity(block.pipelines.len());
        for (index, pipeline) in block.pipelines.iter().enumerate() {
            let span = self.statement_span(pipeline);
            let input_type = if index == 0 { input_type } else { None };
            let mut target = StatementTarget::Block { input_type };
            self.statement_or_classic(pipeline, span, &mut target, &mut out.pipelines);
        }
        Ok(finish_block(
            self.working_set,
            out,
            span,
            scoped,
            is_subexpression,
            input_type,
        ))
    }

    /// The span of a statement in the working set, with its doc comments and the comment after
    /// it on its line.
    pub(in crate::winnow) fn statement_span(&self, pipeline: &w::Pipeline<'s>) -> Span {
        let start = pipeline
            .leading_comments
            .first()
            .map_or(pipeline.span.start, |comment| comment.span.start);
        let end = pipeline
            .trailing_comments
            .last()
            .map_or(pipeline.span.end, |comment| {
                comment.span.end.max(pipeline.span.end)
            });
        self.span(WSpan::new(start, end))
    }
}

/// Parse the statements covering `span` with the classic parser into `out`, with their effects
/// on the working set and, in a module's body, on the module. `is_first` says whether the first
/// of them is the block's first statement, which receives the block's input.
pub(in crate::winnow) fn parse_classic(
    working_set: &mut StateWorkingSet,
    span: Span,
    target: &mut StatementTarget<'_>,
    is_first: bool,
    out: &mut Vec<Pipeline>,
) {
    // With the end of line after it, as in the block: a `|` ending the statement is reported
    // only at the end of the block (`ls |` then a newline is fine).
    let with_eol = Span::new(span.start, span.end + 1);
    let contents = match working_set.get_span_contents(with_eol) {
        contents if contents.len() == with_eol.len() && contents.ends_with(b"\n") => contents,
        _ => working_set.get_span_contents(span),
    };
    let (tokens, err) = lex(contents, span.start, &[], &[], false);
    if let Some(err) = err {
        working_set.error(err);
    }
    let (lite_block, err) = lite_parse(&tokens, working_set);
    if let Some(err) = err {
        working_set.error(err);
    }
    out.reserve(lite_block.block.len());
    for (index, lite_pipeline) in lite_block.block.iter().enumerate() {
        let pipeline = match target {
            StatementTarget::Block { input_type } => {
                let input_type = if is_first && index == 0 {
                    *input_type
                } else {
                    None
                };
                parse_pipeline(working_set, lite_pipeline, input_type)
            }
            StatementTarget::Module { name, module, span } => {
                parse_module_pipeline(working_set, lite_pipeline, name, module, *span)
            }
        };
        out.push(pipeline);
    }
}

/// Whether a statement changes which commands exist for the statements after it.
fn changes_commands(pipeline: &w::Pipeline<'_>) -> bool {
    pipeline
        .elements
        .iter()
        .any(|element| match &element.expr.expr {
            w::Expr::Use(_) | w::Expr::Module(_) | w::Expr::Export(_) => true,
            w::Expr::Call(call) => {
                let head = call.head.name.as_ref();
                head.starts_with("overlay ")
                    || matches!(head, "source" | "source-env" | "hide" | "plugin use")
            }
            _ => false,
        })
}

fn redirection_source(source: WinnowSource) -> RedirectionSource {
    match source {
        WinnowSource::Stdout => RedirectionSource::Stdout,
        WinnowSource::Stderr => RedirectionSource::Stderr,
        WinnowSource::StdoutAndStderr => RedirectionSource::StdoutAndStderr,
    }
}
