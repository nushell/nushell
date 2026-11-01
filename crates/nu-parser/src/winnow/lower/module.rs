//! Module statements: `module`, `use` and their `export`, `hide`, and the overlay commands (the
//! classic parser's `parse_module`, `parse_use`, `parse_hide`, `parse_keyword` and
//! `parse_overlay_*`, in `parse_module.rs` and `parse_keywords.rs`).

use nu_protocol::{
    DeclId, Module, ModuleId, Span, Type,
    ast::{Call, Expr, Pipeline},
    engine::StateWorkingSet,
};

use super::{Lower, Lowered, Unlowered};
use crate::winnow::syntax::ast as w;
use crate::{
    exportable::Exportable,
    parse_calls::CallKind,
    parse_helpers::garbage,
    parse_keywords::find_keyword_decl,
    parse_module::{
        exported_module, finish_hide, finish_module, finish_use, parse_overlay_hide,
        parse_overlay_new, parse_overlay_use, warp_export_call,
    },
};

/// What a parser keyword does to the working set once its call is parsed (`parse_overlay_use`,
/// `parse_plugin_use`, ...), giving the statement.
pub(super) type KeywordEffect = fn(&mut StateWorkingSet, Box<Call>) -> Pipeline;

/// The effect of a call to an overlay command that changes the overlays, by the command's name.
pub(super) fn overlay_effect(name: &str) -> Option<KeywordEffect> {
    match name {
        "overlay use" => Some(parse_overlay_use),
        "overlay hide" => Some(parse_overlay_hide),
        "overlay new" => Some(parse_overlay_new),
        _ => None,
    }
}

impl<'s> Lower<'_, '_, 's> {
    /// `hide name [members]` (`parse_hide`), the statement `e`: the call, its arguments read with
    /// the signature's shapes, then `finish_hide`, which hides what the import pattern names. A
    /// call for help is the statement as it is.
    pub(super) fn hide(&mut self, e: &w::Expression<'s>, call: &w::Call<'s>) -> Lowered<Pipeline> {
        let decl_id = self
            .working_set
            .find_decl(b"hide")
            .ok_or(Unlowered::Unsupported("hide that is not a command"))?;
        let head = self.span(call.head.span);
        let (lowered, output, kind) = self.internal_call(head, &call.arguments, decl_id, None)?;
        let call_span = self.span(e.span);
        if kind != CallKind::Valid {
            let statement = self.node(Expr::Call(lowered), call_span, output);
            return Ok(Pipeline::from_vec(vec![statement]));
        }
        let args_spans: Vec<Span> = call
            .arguments
            .iter()
            .map(|argument| self.span(argument.span()))
            .collect();
        Ok(finish_hide(
            self.working_set,
            lowered,
            call_span,
            &args_spans,
        ))
    }

    /// A parser keyword's statement that `parse_keyword` parses (`overlay use`, `plugin use`,
    /// any other `overlay` statement, ...), `e`: the call, then, unless it asks for help, the
    /// keyword's `effect` on the working set; without an effect, the call as `parse_call` gives
    /// it. A call for help is typed `any`.
    pub(super) fn keyword_statement(
        &mut self,
        e: &w::Expression<'s>,
        call: &w::Call<'s>,
        effect: Option<KeywordEffect>,
    ) -> Lowered<Pipeline> {
        let decl_id = self
            .working_set
            .find_decl(call.head.name.as_bytes())
            .ok_or(Unlowered::Error)?;
        let head = self.span(call.head.span);
        let errors = self.working_set.parse_errors.len();
        let (lowered, output, kind) = self.internal_call(head, &call.arguments, decl_id, None)?;
        // With an error in the call, `parse_keyword` gives the call as `parse_call` makes it.
        if self.working_set.parse_errors.len() > errors {
            let span = self.span(e.span);
            let statement = self.node(Expr::Call(lowered), span, output);
            return Ok(Pipeline::from_vec(vec![statement]));
        }
        if kind == CallKind::Help {
            let span = lowered.span();
            let statement = self.node(Expr::Call(lowered), span, Type::Any);
            return Ok(Pipeline::from_vec(vec![statement]));
        }
        match effect {
            Some(effect) => Ok(effect(self.working_set, lowered)),
            None => {
                let span = self.span(e.span);
                Ok(Pipeline::from_vec(vec![self.node(
                    Expr::Call(lowered),
                    span,
                    output,
                )]))
            }
        }
    }

    /// `module name { body }` or `module path` (`parse_module`), or `export module` when
    /// `command` is an `export`: the call to `module`, which reads only the name, as a
    /// string, and leaves the body garbage (`ArgumentParsingLevel::FirstK`), then
    /// `finish_module`, which parses the module's file or its body (`parse_module_block`) and
    /// adds the module, and for `export module` the call made a call to `export module`
    /// (`warp_export_call`). In a module named `parent_name`, an `export module` exports the
    /// module (`parse_export_in_module`), which must not have the parent's name; the exports
    /// are returned with the statement.
    ///
    /// As for `use`, once `finish_module` has run the statement is never handed back: it
    /// changed the working set, and a module file parsed the first time would not report its
    /// errors again.
    pub(super) fn module_declaration(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        module: &w::Module<'s>,
        parent_name: Option<&[u8]>,
    ) -> Lowered<(Pipeline, Vec<Exportable>)> {
        let (decl_id, head) = self.module_keyword(command)?;
        let body_span = match &module.body {
            Some(body) => Some(self.braces_span(body)?),
            None => None,
        };
        let name = w::Argument::Positional((*module.name).clone());
        let (mut call, _, kind) = self.internal_call(head, &[name], decl_id, None)?;
        // `parse_module` gives an invalid call (a missing positional) back as it is; the
        // winnow parser reads a `module` without a name as a call (`module_call`), so none
        // reaches here.
        if kind == CallKind::Invalid {
            return Err(Unlowered::Error);
        }
        if let Some(body_span) = body_span {
            let span = self.span(body_span);
            call.add_positional(garbage(self.working_set, span));
        }
        let body_span = body_span.map(|span| self.span(span));
        Ok(self.finish_module_statement(pipeline, command, head, call, body_span, parent_name))
    }

    /// `module` read as the call `parse_module` makes first, `call` (the winnow parser's call
    /// form of a `module` statement: a call for help, a flag, a name that is no literal, an
    /// item in the body's place that is no block, ...), or the `export` of one, `command`: its
    /// first positional read with its shape, every later one garbage, which the winnow parser
    /// left unread (`ArgumentParsingLevel::FirstK { k: 1 }`), its errors reported as a call's. A call for
    /// help or one missing its name is the statement. Any other goes on as a declaration whose
    /// body is the item after the name, whatever it is (`parse_module`'s `spans[split_id +
    /// 1]`): `module x --foo {}` has `--foo` there, which `finish_module` refuses as no block,
    /// while `module x {} --foo` adds the module.
    pub(super) fn module_call(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        call: &w::Call<'s>,
        parent_name: Option<&[u8]>,
    ) -> Lowered<(Pipeline, Vec<Exportable>)> {
        // A redirected `module` is an error (`redirecting_builtin_error`), the classic parser's.
        if pipeline
            .elements
            .iter()
            .any(|element| element.redirection.is_some())
        {
            return Err(Unlowered::Error);
        }
        let (decl_id, head) = self.module_keyword(command)?;
        let (lowered, output, kind) = self.internal_call(head, &call.arguments, decl_id, None)?;
        if kind == CallKind::Valid {
            let body_span = call
                .arguments
                .get(1)
                .map(|argument| self.span(argument.span()));
            return Ok(self.finish_module_statement(
                pipeline,
                command,
                head,
                lowered,
                body_span,
                parent_name,
            ));
        }
        let call_span = Span::new(head.start, self.span(command.span).end);
        let statement = Pipeline::from_vec(vec![self.node(Expr::Call(lowered), call_span, output)]);
        Ok(self.export_module(command, head, call_span, statement, None, parent_name))
    }

    /// The declaration of `module` and its head for `command`, a `module` statement or the
    /// `export` of one, whose keyword must be `export module` then.
    fn module_keyword(&mut self, command: &w::Expression<'s>) -> Lowered<(DeclId, Span)> {
        let decl_id = find_keyword_decl(self.working_set, b"module")
            .ok_or(Unlowered::Unsupported("module that is not a keyword"))?;
        if matches!(command.expr, w::Expr::Export(_))
            && find_keyword_decl(self.working_set, b"export module").is_none()
        {
            return Err(Unlowered::Unsupported(
                "export module that is not a keyword",
            ));
        }
        Ok((decl_id, self.command_head(command, "module")))
    }

    /// `finish_module` over the call to `module` of `command` and the item `body_span` in the
    /// body's place, then the `export` around it ([`Self::export_module`]).
    fn finish_module_statement(
        &mut self,
        pipeline: &w::Pipeline<'s>,
        command: &w::Expression<'s>,
        head: Span,
        call: Box<Call>,
        body_span: Option<Span>,
        parent_name: Option<&[u8]>,
    ) -> (Pipeline, Vec<Exportable>) {
        // The whole statement, a `--` after the body included (which nu drops, and which the
        // span of the `module` after an `export` leaves out).
        let call_span = Span::new(head.start, self.span(command.span).end);
        let comments = self.doc_comments(pipeline, &[], command);
        let (statement, module_id) = finish_module(
            self.working_set,
            call,
            call_span,
            body_span,
            parent_name,
            comments,
        );
        self.export_module(command, head, call_span, statement, module_id, parent_name)
    }

    /// For an `export module`, `statement` made a call to `export module` covering the statement
    /// (`warp_export_call`) and, in a module named `parent_name`, the module it added,
    /// `module_id`, exported (`parse_export_in_module`). Returns the statement and the exports.
    fn export_module(
        &mut self,
        command: &w::Expression<'s>,
        head: Span,
        call_span: Span,
        mut statement: Pipeline,
        module_id: Option<ModuleId>,
        parent_name: Option<&[u8]>,
    ) -> (Pipeline, Vec<Exportable>) {
        if !matches!(command.expr, w::Expr::Export(_)) {
            return (statement, Vec::new());
        }
        // `export`, `module`, and the rest of the statement, as `warp_export_call` takes them.
        let keyword = Span::new(head.end - "module".len(), head.end);
        let spans = [
            Span::new(head.start, head.start + "export".len()),
            keyword,
            Span::new(call_span.end, call_span.end),
        ];
        warp_export_call(self.working_set, &mut statement, "export module", &spans);
        let exportables = match parent_name {
            Some(_) => exported_module(self.working_set, module_id),
            None => Vec::new(),
        };
        (statement, exportables)
    }

    /// `use module members` (`parse_use`), or `export use` when `command` is an `export` of
    /// `item`: the call to `use`, its arguments read with the signature's shapes
    /// (`parse_internal_call`), then `finish_use`, which finds the module (parsing its file)
    /// and brings the members into scope, and for `export use` the call made a call to `export
    /// use` (`warp_export_call`). In a module's body, `parent_module` keeps track of the
    /// modules the statement imports. Returns the statement and what it brought into scope,
    /// which an `export use` in a module exports.
    ///
    /// `finish_use` changes the working set and reports the classic parser's own errors, so
    /// once it has run the statement is never handed back: the classic parser would do it all
    /// again, and a module file parsed the first time would not report its errors again.
    pub(super) fn use_statement(
        &mut self,
        command: &w::Expression<'s>,
        item: &w::Expression<'s>,
        use_statement: &w::Use<'s>,
        parent_module: Option<&mut Module>,
    ) -> Lowered<(Pipeline, Vec<Exportable>)> {
        let export = matches!(command.expr, w::Expr::Export(_));
        let decl_id = find_keyword_decl(self.working_set, b"use")
            .ok_or(Unlowered::Unsupported("use that is not a keyword"))?;
        if export && find_keyword_decl(self.working_set, b"export use").is_none() {
            return Err(Unlowered::Unsupported("export use that is not a keyword"));
        }
        // `parse_export_in_module` parses an `export use` from its `use`, and
        // `parse_export_in_block` from its `export`.
        let call_start = if parent_module.is_some() {
            item
        } else {
            command
        };
        let head = self.command_head(call_start, "use");
        let name_span = Span::new(head.end - "use".len(), head.end);
        // The items after `use` in source order, the `--` that ends the options included.
        let mut arguments: Vec<w::Argument<'s>> =
            std::iter::once(w::Argument::Positional((*use_statement.module).clone()))
                .chain(
                    use_statement
                        .members
                        .iter()
                        .map(|member| w::Argument::Positional(member.value.clone())),
                )
                .chain(use_statement.end_of_options.map(w::Argument::EndOfOptions))
                .chain(use_statement.flags.iter().cloned())
                .collect();
        arguments.sort_by_key(|argument| argument.span().start);
        let args_spans: Vec<Span> = arguments
            .iter()
            .map(|argument| self.span(argument.span()))
            .collect();
        let end = args_spans.last().map_or(head.end, |span| span.end);
        let (call, _, kind) = self.internal_call(head, &arguments, decl_id, None)?;
        // `parse_use` gives an invalid call (a missing positional) back as it is; the winnow
        // parser reports a `use` without a module, so none reaches here.
        if kind == CallKind::Invalid {
            return Err(Unlowered::Error);
        }
        let (mut pipeline, exportables) = finish_use(
            self.working_set,
            call,
            name_span,
            Span::new(head.start, end),
            &args_spans,
            parent_module,
        );
        if export {
            // `export`, `use`, and the rest of the statement, as `warp_export_call` takes them.
            let start = self.span(command.span).start;
            let spans = [
                Span::new(start, start + "export".len()),
                name_span,
                Span::new(end, end),
            ];
            warp_export_call(self.working_set, &mut pipeline, "export use", &spans);
        }
        Ok((pipeline, exportables))
    }
}
