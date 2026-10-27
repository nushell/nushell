//! Module statements: `use` and `export use` (the classic parser's `parse_use`, in
//! `parse_module.rs`).

use nu_protocol::{Module, Span, ast::Pipeline};

use super::{Lower, Lowered, Unlowered};
use crate::winnow::syntax::ast as w;
use crate::{
    exportable::Exportable,
    parse_keywords::find_keyword_decl,
    parse_module::{finish_use, warp_export_call},
};

impl<'s> Lower<'_, '_, 's> {
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
                .collect();
        arguments.sort_by_key(|argument| argument.span().start);
        let args_spans: Vec<Span> = arguments
            .iter()
            .map(|argument| self.span(argument.span()))
            .collect();
        let end = args_spans.last().map_or(head.end, |span| span.end);
        let (call, _) = self.internal_call(head, &arguments, decl_id, None)?;
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
