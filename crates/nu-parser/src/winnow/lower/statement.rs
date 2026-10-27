//! Blocks, statements and pipelines (the classic parser's `parse_block`,
//! `parse_builtin_commands`, `parse_pipeline` and `parse_pipeline_element`), and the hand-over
//! of a statement to the classic parser.

use nu_protocol::{
    DeclId, Module, Span, SyntaxShape, Type,
    ast::{
        Block, Call, Expr, Expression, Pipeline, PipelineElement, PipelineRedirection,
        RedirectionSource, RedirectionTarget,
    },
    engine::StateWorkingSet,
};

use super::module::{KeywordEffect, overlay_effect};
use super::{Lower, Lowered, Unlowered};
#[cfg(feature = "plugin")]
use crate::parse_source::parse_plugin_use;
use crate::winnow::syntax::lex::RedirectionSource as WinnowSource;
use crate::winnow::syntax::{Span as WSpan, ast as w};
use crate::{
    exportable::Exportable,
    lex::lex,
    lite_parser::lite_parse,
    parse_captures_compile::wrap_element_with_collect,
    parse_helpers::{PERCENT_FORCED_BUILTIN_PARSER_INFO, trim_quotes},
    parse_module::{add_exportables, double_main_span, parse_module_pipeline},
    parse_pipelines::{finish_block, hide_unlet_constants, parse_pipeline},
    winnow::{
        driver::{BlockKind, parse_block, statement_changes_names},
        stats,
    },
};

/// Where a statement goes: an ordinary block, or a module's body.
pub(in crate::winnow) enum StatementTarget<'t> {
    /// A statement of an ordinary block; `input_type` is the block's input, which only its
    /// first statement receives. `is_subexpression` is the classic `parse_block`'s flag: the
    /// classic parser then lexes the statement as `parse_full_cell_path` lexes a parenthesized
    /// subexpression, newlines as whitespace. (The value of a binding or an assignment is also
    /// a subexpression; it is one pipeline, which lexes the same either way.)
    Block {
        input_type: Option<&'t Type>,
        is_subexpression: bool,
    },
    /// A statement of the body of the module `name`, covering `span`; exports add to `module`.
    Module {
        name: &'t [u8],
        module: &'t mut Module,
        span: Span,
    },
}

impl<'s> Lower<'_, '_, 's> {
    /// A statement lowered, or parsed by the classic parser from `span` when the lowering gives
    /// it back, added to `out`. The errors,
    /// warnings and compile errors the lowering reported before giving up are dropped, and so
    /// are the constants an `unlet` it lowered hid in the current scope (which the classic
    /// parser must still find, in the `unlet` itself); what else it added to the working set
    /// stays, unreachable from `out` (see the module documentation of `lower`). `block_errors`
    /// is where the block's errors start ([`parse_classic`]).
    pub(in crate::winnow) fn statement_or_classic(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        span: Span,
        target: &mut StatementTarget<'_>,
        out: &mut Vec<Pipeline>,
        block_errors: &mut usize,
    ) {
        let errors = self.working_set.parse_errors.len();
        let warnings = self.working_set.parse_warnings.len();
        let compile_errors = self.working_set.compile_errors.len();
        // Empty, and so free to copy, unless an `unlet` in the block hid a constant.
        let hidden_vars = self
            .working_set
            .delta
            .last_scope_frame_mut()
            .hidden_vars
            .clone();
        match self.statement(pipeline, target) {
            Ok(pipeline) => {
                stats::record_lowered_statement();
                out.push(pipeline);
            }
            Err(reason) => {
                self.working_set.parse_errors.truncate(errors);
                self.working_set.parse_warnings.truncate(warnings);
                self.working_set.compile_errors.truncate(compile_errors);
                self.working_set.delta.last_scope_frame_mut().hidden_vars = hidden_vars;
                stats::record_unlowered(span, &reason, self.working_set);
                parse_classic(self.working_set, span, target, out, block_errors);
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
            StatementTarget::Block { input_type, .. } => self.pipeline(pipeline, *input_type),
            StatementTarget::Module { name, module, span } => {
                self.module_statement(pipeline, name, module, *span)
            }
        }
    }

    /// A statement of a module's body (`parse_module_pipeline`): definitions, constants,
    /// aliases, modules, `use` and `export-env` blocks, and their exports added to `module`,
    /// whose body covers `span`. `module` changes only once nothing can fail, since the classic
    /// parser adds the exports of a statement given back.
    fn module_statement(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        module_name: &[u8],
        module: &mut Module,
        span: Span,
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
                    .0
            }
            w::Expr::Const(binding) if attributes.is_empty() => {
                self.const_statement(command, binding)?.0
            }
            w::Expr::ExportEnv(export_env) if attributes.is_empty() => {
                let (expression, block_id) = self.export_env(command, export_env)?;
                module.add_env_block(block_id);
                expression
            }
            w::Expr::Alias(alias) if attributes.is_empty() => {
                self.alias(pipeline, command, alias, None)?
            }
            w::Expr::TypeAlias(alias) if attributes.is_empty() => {
                return Ok(self.type_alias(command, alias)?.0);
            }
            w::Expr::Use(use_statement) if attributes.is_empty() => {
                let (pipeline, _) =
                    self.use_statement(command, command, use_statement, Some(module))?;
                return Ok(pipeline);
            }
            w::Expr::Module(submodule) if attributes.is_empty() => {
                let (pipeline, _) = self.module_declaration(pipeline, command, submodule, None)?;
                return Ok(pipeline);
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
                    let (expression, decl) = self.extern_def(
                        pipeline,
                        command,
                        extern_def,
                        attributes,
                        Some(module_name),
                    )?;
                    if let Some((name, decl_id)) = decl {
                        self.add_export_decl(module, module_name, name, decl_id, &expression);
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
                w::Expr::Alias(alias) if attributes.is_empty() => {
                    self.export_alias(pipeline, command, &export.item, alias, module_name, module)?
                }
                w::Expr::TypeAlias(alias) if attributes.is_empty() => {
                    let (pipeline, exportables) = self.type_alias(command, alias)?;
                    let main_span =
                        double_main_span(pipeline.elements.first().map(|e| &e.expr), span);
                    add_exportables(
                        self.working_set,
                        module,
                        module_name,
                        exportables,
                        main_span,
                    );
                    return Ok(pipeline);
                }
                w::Expr::Use(use_statement) if attributes.is_empty() => {
                    let (pipeline, exportables) =
                        self.use_statement(command, &export.item, use_statement, Some(module))?;
                    let main_span =
                        double_main_span(pipeline.elements.first().map(|e| &e.expr), span);
                    add_exportables(
                        self.working_set,
                        module,
                        module_name,
                        exportables,
                        main_span,
                    );
                    return Ok(pipeline);
                }
                w::Expr::Module(submodule) if attributes.is_empty() => {
                    let (pipeline, exportables) =
                        self.module_declaration(pipeline, command, submodule, Some(module_name))?;
                    let main_span =
                        double_main_span(pipeline.elements.first().map(|e| &e.expr), span);
                    add_exportables(
                        self.working_set,
                        module,
                        module_name,
                        exportables,
                        main_span,
                    );
                    return Ok(pipeline);
                }
                _ => return Err(Unlowered::Unsupported("export in a module")),
            },
            _ => return Err(Unlowered::Unsupported("module statement")),
        };
        Ok(Pipeline::from_vec(vec![expression]))
    }

    /// `export alias name = target` in a module's body (`parse_export_in_module`): the alias,
    /// declared as `alias` declares it with the module's name forbidden, made a call to `export
    /// alias` covering the statement (`warp_export_call`), and added to the module's exports
    /// under the item after `alias`, its quotes trimmed. An alias that reports an error is the
    /// classic parser's.
    fn export_alias(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        item: &w::Expression<'s>,
        alias: &w::Alias<'s>,
        module_name: &[u8],
        module: &mut Module,
    ) -> Lowered<Expression> {
        let errors = self.working_set.parse_errors.len();
        let mut expression = self.alias(pipeline, item, alias, Some(module_name))?;
        if self.working_set.parse_errors.len() > errors {
            return Err(Unlowered::Error);
        }
        let span = self.span(command.span);
        self.export_call(&mut expression, "export alias", span)?;
        // The head covers `export alias`, the blanks between them included.
        if let Expr::Call(call) = &mut expression.expr {
            call.head = Span::new(span.start, self.span(item.span).start + "alias".len());
        }
        let name = trim_quotes(self.text(alias.name.span).as_bytes()).to_vec();
        let decl_id = self.working_set.find_decl(&name).ok_or(Unlowered::Error)?;
        self.add_export_decl(module, module_name, name, decl_id, &expression);
        Ok(expression)
    }

    /// Add an exported command, the statement `expression`, to a module: as its `main`, or
    /// under its name (`add_exportables`).
    fn add_export_decl(
        &mut self,
        module: &mut Module,
        module_name: &[u8],
        name: Vec<u8>,
        decl_id: DeclId,
        expression: &Expression,
    ) {
        let main_span = double_main_span(Some(expression), expression.span);
        let exportable = Exportable::Decl { name, id: decl_id };
        add_exportables(
            self.working_set,
            module,
            module_name,
            vec![exportable],
            main_span,
        );
    }

    /// `export const` (in a script or a module): the constant, made a call to `export const`
    /// covering the statement. Returns it with the variable's name span.
    fn export_const(
        &mut self,
        command: &w::Expression<'s>,
        export: &w::Export<'s>,
        binding: &w::Binding<'s>,
    ) -> Lowered<(Expression, Span)> {
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
                    .0
            }
            w::Expr::Const(binding) => self.const_statement(&element.expr, binding)?.0,
            w::Expr::ExportEnv(export_env) => self.export_env(&element.expr, export_env)?.0,
            // `export def`, `export extern` and `export const` outside a module
            // (`parse_export_in_block`).
            w::Expr::Export(export) => match &export.item.expr {
                w::Expr::Def(def) => self.def(pipeline, &element.expr, def, &[], None)?.0,
                w::Expr::Extern(extern_def) => {
                    self.extern_def(pipeline, &element.expr, extern_def, &[], None)?
                        .0
                }
                w::Expr::Const(binding) => self.export_const(&element.expr, export, binding)?.0,
                // `parse_alias` with `export alias` its head, made a call to `export alias`.
                w::Expr::Alias(alias) => {
                    let mut expression = self.alias(pipeline, &element.expr, alias, None)?;
                    let span = self.span(element.expr.span);
                    self.export_call(&mut expression, "export alias", span)?;
                    expression
                }
                w::Expr::Use(use_statement) => {
                    let (pipeline, _) =
                        self.use_statement(&element.expr, &export.item, use_statement, None)?;
                    return Ok(pipeline);
                }
                w::Expr::Module(module) => {
                    let (pipeline, _) =
                        self.module_declaration(pipeline, &element.expr, module, None)?;
                    return Ok(pipeline);
                }
                w::Expr::TypeAlias(alias) => return Ok(self.type_alias(&element.expr, alias)?.0),
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
                            .0
                    }
                    _ => return Err(Unlowered::Unsupported("attributes")),
                }
            }
            w::Expr::Let(binding) => return self.let_statement(element, binding, input_type),
            w::Expr::Mut(binding) => return self.mut_statement(element, binding),
            w::Expr::For(for_loop) => self.for_statement(element, for_loop)?,
            w::Expr::Alias(alias) => self.alias(pipeline, &element.expr, alias, None)?,
            w::Expr::TypeAlias(alias) => return Ok(self.type_alias(&element.expr, alias)?.0),
            w::Expr::Module(module) => {
                let (pipeline, _) =
                    self.module_declaration(pipeline, &element.expr, module, None)?;
                return Ok(pipeline);
            }
            w::Expr::Use(use_statement) => {
                let (pipeline, _) =
                    self.use_statement(&element.expr, &element.expr, use_statement, None)?;
                return Ok(pipeline);
            }
            // The parser keywords the classic parser picks by the statement's first words.
            w::Expr::Call(call) if call.sigil.is_none() => {
                match call.head.name.as_ref() {
                    "source" | "source-env" => return self.source(&element.expr, call),
                    "hide" => return self.hide(&element.expr, call),
                    "run" => return Ok(Pipeline::from_vec(vec![self.run(&element.expr, call)?])),
                    #[cfg(feature = "plugin")]
                    "plugin use" => return self.keyword_statement(call, parse_plugin_use),
                    name => {
                        if let Some(effect) = overlay_effect(name) {
                            return self.keyword_statement(call, effect);
                        }
                    }
                }
                return self.single_element(pipeline, element, input_type);
            }
            _ => return self.single_element(pipeline, element, input_type),
        };
        Ok(Pipeline::from_vec(vec![expr]))
    }

    /// A pipeline of one command that is no statement (the end of `parse_builtin_commands`). A
    /// call to `overlay use`, `overlay hide` or `overlay new` through an alias changes the
    /// overlays as a call to the command itself does (`parse_overlay_use`, ...): the statement
    /// is what that gives, without the element's redirection.
    fn single_element(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        element: &w::PipelineElement<'s>,
        input_type: Option<&Type>,
    ) -> Lowered<Pipeline> {
        let pipe = self.classic_pipes(pipeline)[0];
        let element = self.pipeline_element(element, input_type.unwrap_or(&Type::Any), pipe)?;
        if let Expr::Call(call) = &element.expr.expr
            && let Some(effect) = self.overlay_call_effect(call)
        {
            return Ok(effect(self.working_set, call.clone()));
        }
        Ok(Pipeline {
            elements: vec![element],
        })
    }

    /// Whether `expression` calls `overlay use`, `overlay hide` or `overlay new`, maybe through
    /// an alias: the classic `parse_builtin_commands` applies such a call's effect on the
    /// scope as it parses it.
    fn calls_overlay(&self, expression: &Expression) -> bool {
        matches!(
            &expression.expr,
            Expr::Call(call) if self.overlay_call_effect(call).is_some()
        )
    }

    /// The effect on the overlays of `call` when it calls `overlay use`, `overlay hide` or
    /// `overlay new`, maybe through an alias. A dynamic `%` call has none: its declaration is a
    /// placeholder until the IR compiler names the command (`parse_builtin_commands`).
    fn overlay_call_effect(&self, call: &Call) -> Option<KeywordEffect> {
        if call
            .parser_info
            .contains_key(PERCENT_FORCED_BUILTIN_PARSER_INFO)
        {
            return None;
        }
        overlay_effect(self.working_set.get_decl(call.decl_id).name())
    }

    /// The `pipe` of each element as the classic lite parser sets it. The winnow parser keeps,
    /// with each element, the last `|` before it; the classic lite parser keeps the `|` that
    /// ends an element, the first one after it:
    ///
    /// ```text
    /// ls | sort | first
    ///    a      b
    /// winnow:  ls -, sort a, first b      (- for none)
    /// classic: ls a, sort b, first -
    /// ```
    ///
    /// An element ending with `e>|` or `o+e>|` keeps it: the classic lite parser's
    /// `ErrGreaterPipe`/`OutErrGreaterPipe` arms end the command there as a `|` does.
    ///
    /// The last element, which no `|` ends, keeps a `|` after it that no command follows
    /// (`ls |` at the end of a block, [`w::Pipeline::dangling_pipe`]), or else the last of two
    /// or more `|` before it (and the
    /// first element a leading `|`), which empty commands between them hand on; an `e>|` that
    /// ends the element before counts as one of them (`^ls e>|\n| lines`).
    fn classic_pipes(&self, pipeline: &w::Pipeline<'s>) -> Vec<Option<WSpan>> {
        let elements = &pipeline.elements;
        let last = elements.len() - 1;
        (0..elements.len())
            .map(|index| {
                if let Some(op) = redirection_pipe(&elements[index]) {
                    Some(op)
                } else if index < last {
                    let next = &elements[index + 1];
                    let gap_end = next.pipe.map_or(next.span.start, |pipe| pipe.start);
                    self.first_pipe(elements[index].span.end, gap_end)
                        .or(next.pipe)
                } else if let Some(dangling) = pipeline.dangling_pipe {
                    Some(dangling)
                } else {
                    let pipe = elements[index].pipe?;
                    let previous = match index {
                        0 => return Some(pipe),
                        _ => &elements[index - 1],
                    };
                    // `pipe` is the `e>|` itself when no `|` follows it.
                    let after_redirection_pipe =
                        redirection_pipe(previous).is_some_and(|op| op != pipe);
                    (after_redirection_pipe
                        || self.first_pipe(previous.span.end, pipe.start).is_some())
                    .then_some(pipe)
                }
            })
            .collect()
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
    /// the `|` the classic parser keeps with it. An `unlet` of a constant hides it from here on,
    /// as there.
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
        hide_unlet_constants(self.working_set, &expr);
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

    /// Where a redirection goes (`parse_redirection_target`): a file, whose path is read with
    /// any shape, or the next command (`e>|`, `o+e>|`).
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
    /// gives back are parsed by the classic parser, and so is the rest of the block after one
    /// of them that called `overlay use` (or `hide`, `new`) through an alias.
    pub(super) fn block(
        &mut self,
        block: &w::Block<'s>,
        span: Span,
        scoped: bool,
        is_subexpression: bool,
        input_type: Option<&Type>,
    ) -> Lowered<Block> {
        // In an alias's target, a block's statements are statements again.
        let outer_alias_target = std::mem::replace(&mut self.alias_target, false);
        let lowered = self.block_statements(block, span, scoped, is_subexpression, input_type);
        self.alias_target = outer_alias_target;
        lowered
    }

    /// [`Self::block`], outside an alias's target.
    fn block_statements(
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
        // statement at a time, as a file is. The winnow parser leaves such a block unparsed,
        // empty (`CommandLookup::defers_command_changing_blocks`).
        let unparsed = block.pipelines.is_empty() && !self.text(block.span).trim().is_empty();
        if unparsed || block.pipelines.iter().any(changes_commands) {
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
        // Where the block's errors start (`parse_classic`).
        let mut block_errors = self.working_set.parse_errors.len();
        if scoped {
            self.working_set.enter_scope();
        }
        // A definition the lowering cannot predeclare leaves the whole block, with the
        // statement it is in, to the classic parser, which predeclares it.
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
            let mut target = StatementTarget::Block {
                input_type,
                is_subexpression,
            };
            let first = out.pipelines.len();
            self.statement_or_classic(
                pipeline,
                span,
                &mut target,
                &mut out.pipelines,
                &mut block_errors,
            );
            // A statement may call `overlay use` through an alias (which `changes_commands`
            // cannot tell from its head), which changed the overlays as it was lowered or as
            // the classic parser parsed it: the rest of the block, parsed before with the old
            // commands, is parsed by the classic parser, as the driver's `rest_to_classic` does.
            if let Some(next) = block.pipelines.get(index + 1)
                && out.pipelines[first..]
                    .iter()
                    .flat_map(|pipeline| &pipeline.elements)
                    .any(|element| self.calls_overlay(&element.expr))
            {
                let rest = Span::new(self.statement_span(next).start, self.span(block.span).end);
                let reason = Unlowered::Unsupported("rest of a block after an overlay");
                stats::record_unlowered(rest, &reason, self.working_set);
                let mut target = StatementTarget::Block {
                    input_type: None,
                    is_subexpression,
                };
                parse_classic(
                    self.working_set,
                    rest,
                    &mut target,
                    &mut out.pipelines,
                    &mut block_errors,
                );
                break;
            }
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

    /// The span of a statement in the working set, with its doc comments, the comment after it
    /// on its line and a `|` after it that no command follows, which the classic parser keeps
    /// with its last element.
    ///
    /// A doc comment after the `;` of the statement before (`a; # doc`) is the statement's
    /// because the classic lite parser reads it after that `;`; read from the comment on, it is
    /// dropped at a blank line before the statement. The span then starts at the `;`.
    pub(in crate::winnow) fn statement_span(&self, pipeline: &w::Pipeline<'s>) -> Span {
        let start = pipeline
            .leading_comments
            .first()
            .map_or(pipeline.span.start, |comment| {
                let before = self.source[..comment.span.start].trim_end_matches([' ', '\t']);
                match before.strip_suffix(';') {
                    Some(rest) => rest.len(),
                    None => comment.span.start,
                }
            });
        let end = pipeline
            .trailing_comments
            .last()
            .map(|comment| comment.span.end)
            .into_iter()
            .chain(pipeline.dangling_pipe.map(|pipe| pipe.end))
            .fold(pipeline.span.end, usize::max);
        self.span(WSpan::new(start, end))
    }
}

/// Parse the statements covering `span` with the classic parser into `out`, with their effects
/// on the working set and, in a module's body, on the module. The first of them receives the
/// target's input, which the caller gives only to a block's first statement. Unlike the classic
/// `parse_block`, it predeclares no definitions: the block's are declared already.
///
/// The classic parser lexes and lite-parses a whole block before it parses any statement, so it
/// reports those errors (a `||`, a redirection without a target) before the errors of statements
/// before them: they go to `block_errors`, where the block's errors start, which moves past them.
pub(in crate::winnow) fn parse_classic(
    working_set: &mut StateWorkingSet,
    span: Span,
    target: &mut StatementTarget<'_>,
    out: &mut Vec<Pipeline>,
    block_errors: &mut usize,
) {
    let subexpression = matches!(
        target,
        StatementTarget::Block {
            is_subexpression: true,
            ..
        }
    );
    let (tokens, err) = if subexpression {
        // As `parse_full_cell_path` lexes a parenthesized subexpression: newlines are
        // whitespace and comments are skipped, so a statement goes on over lines.
        lex(
            working_set.get_span_contents(span),
            span.start,
            b"\n\r",
            &[],
            true,
        )
    } else {
        // With the newline after it, when only what the lexer skips (spaces, tabs, a lone
        // `\r`) comes before it, as in the block: a `|` ending the statement is reported only
        // at the end of the block (`ls |` then a newline is fine).
        let mut end = span.end;
        while let [b' ' | b'\t' | b'\r'] = working_set.get_span_contents(Span::new(end, end + 1)) {
            end += 1;
        }
        let with_eol = Span::new(span.start, end + 1);
        let contents = match working_set.get_span_contents(with_eol) {
            contents if contents.len() == with_eol.len() && contents.ends_with(b"\n") => contents,
            _ => working_set.get_span_contents(span),
        };
        lex(contents, span.start, &[], &[], false)
    };
    let (lite_block, lite_err) = lite_parse(&tokens, working_set);
    for err in [err, lite_err].into_iter().flatten() {
        let at = (*block_errors).min(working_set.parse_errors.len());
        working_set.parse_errors.insert(at, err);
        *block_errors = at + 1;
    }
    out.reserve(lite_block.block.len());
    for (index, lite_pipeline) in lite_block.block.iter().enumerate() {
        let pipeline = match target {
            StatementTarget::Block { input_type, .. } => {
                let input_type = if index == 0 { *input_type } else { None };
                parse_pipeline(working_set, lite_pipeline, input_type)
            }
            StatementTarget::Module { name, module, span } => {
                parse_module_pipeline(working_set, lite_pipeline, name, module, *span)
            }
        };
        out.push(pipeline);
    }
}

/// Whether a statement changes which commands exist for the statements after it: one the
/// driver ends a run parsed ahead after ([`statement_changes_names`]), a module or an export.
/// A `true` too many costs only parsing the block again ([`Lower::block`]); a statement this
/// misses (a call through an alias of `overlay use`) is caught there once the classic parser
/// has parsed it.
fn changes_commands(pipeline: &w::Pipeline<'_>) -> bool {
    pipeline
        .elements
        .iter()
        .any(|element| matches!(element.expr.expr, w::Expr::Module(_) | w::Expr::Export(_)))
        || statement_changes_names(pipeline)
}

/// The `e>|` or `o+e>|` that ends `element`, if one does (alone, or after an `o> file`).
fn redirection_pipe(element: &w::PipelineElement<'_>) -> Option<WSpan> {
    match &element.redirection {
        Some(
            w::PipelineRedirection::Single {
                target: w::RedirectionTarget::Pipe { op },
                ..
            }
            | w::PipelineRedirection::Separate {
                err: w::RedirectionTarget::Pipe { op },
                ..
            },
        ) => Some(op.span),
        _ => None,
    }
}

/// The stream a redirection takes, as `nu-protocol` names it.
fn redirection_source(source: WinnowSource) -> RedirectionSource {
    match source {
        WinnowSource::Stdout => RedirectionSource::Stdout,
        WinnowSource::Stderr => RedirectionSource::Stderr,
        WinnowSource::StdoutAndStderr => RedirectionSource::StdoutAndStderr,
    }
}
