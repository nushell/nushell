use super::completer::touches;
use crate::{
    FileCompletion,
    completions::{
        Completer, Context, DirectoryCompletion, ExportableCompletion, Fetched, SemanticSuggestion,
        completion_options::NuMatcher, to_reedline_span,
    },
};
use nu_parser::parse_module_file_or_dir;
use nu_protocol::{
    DynamicCompletionCallRef, Span, SyntaxShape,
    ast::{Argument, Call, Expr, Expression, ListItem},
    engine::{ArgType, StateWorkingSet},
};

pub struct ArgValueCompletion<'a> {
    pub call: &'a Call,
    pub arg_type: ArgType<'a>,
    /// Whether to fall back to file completion when no source matches.
    pub need_fallback: bool,
    /// Index into `call.arguments`, or `.len()` for a trailing slot with no argument.
    pub arg_idx: usize,
    /// The `SyntaxShape` this argument is declared with in the command's signature, if
    /// known. Used to pick a type-specific fallback (e.g. directories for `cd <tab>`) when
    /// the slot is still empty and so has no parsed expression to read the shape from.
    pub declared_shape: Option<SyntaxShape>,
    /// Cursor, in absolute working-set (span) coordinates.
    pub cursor: usize,
}

impl<'a> Completer for ArgValueCompletion<'a> {
    fn fetch(&mut self, context: &Context) -> Fetched {
        if let Some(fetched_completion) = self.try_fetch_dynamic_completion(context) {
            return fetched_completion;
        }

        let prefix_string = context.prefix_str();

        let completion_context = Context {
            prefix: prefix_string.as_ref().as_bytes(),
            ..*context
        };

        // Command-specific completions are dispatched earlier via `BuiltinCompletion`;
        // here we handle only the generic argument-value fallbacks.
        self.fetch_fallback_completion(self.arg_expr(), &completion_context)
    }
}

impl<'a> ArgValueCompletion<'a> {
    fn try_fetch_dynamic_completion(&self, context: &Context) -> Option<Fetched> {
        let working_set = context.working_set;
        let declaration = working_set.get_decl(self.call.decl_id);
        let mut stack = context.stack.to_owned();

        let dynamic_completion_call = DynamicCompletionCallRef {
            call: self.call,
            // No sentinel is appended to the buffer, so nothing to strip.
            strip: false,
            pos: self.cursor,
        };

        let completion_result = declaration.get_dynamic_completion(
            working_set.permanent_state,
            &mut stack,
            dynamic_completion_call,
            &self.arg_type,
            #[expect(deprecated, reason = "internal usage")]
            nu_protocol::engine::ExperimentalMarker,
        );

        match completion_result {
            Ok(Some(items)) => {
                let prefix = context.prefix_str();
                let mut matcher = NuMatcher::new(prefix.as_ref(), context.options, true);

                for suggestion_item in items {
                    // The span is untrusted user/plugin input; fall back to the argument's
                    // own span unless it is well-formed and within the working set.
                    let result_span = suggestion_item
                        .span
                        .filter(|span| span.start >= context.offset && span.end >= span.start)
                        .unwrap_or(context.span);
                    let suggestion = SemanticSuggestion::from_dynamic_suggestion(
                        suggestion_item,
                        to_reedline_span(result_span, context.offset),
                        None,
                    );
                    matcher.add_semantic_suggestion(suggestion);
                }

                Some(Fetched::answering(matcher.suggestion_results()).worth_keeping())
            }
            Ok(None) => None, // fallback to type based completion, file completion, etc.
            Err(error) => {
                log::error!(
                    "error on fetching dynamic suggestion on {} with {:?}: {error}",
                    declaration.name(),
                    self.arg_type
                );
                None
            }
        }
    }

    /// The parsed expression of the argument being completed, if any.
    fn arg_expr(&self) -> Option<&Expr> {
        self.call
            .arguments
            .get(self.arg_idx)
            .and_then(|argument| argument.expr())
            .map(|expression_wrapper| &expression_wrapper.expr)
    }

    pub(crate) fn complete_module_exports(
        &self,
        completion_context: &Context,
        working_set: &StateWorkingSet,
    ) -> Fetched {
        let expression = self.arg_expr();
        let Some((module_name, span)) = self.find_module_name_and_span() else {
            return Fetched::answering(vec![]);
        };

        let Some((module_id, temp_working_set)) =
            self.resolve_module(working_set, module_name, span)
        else {
            return Fetched::answering(vec![]);
        };

        let mut exportable_completion = ExportableCompletion {
            module_id,
            temp_working_set,
        };

        // Reaching this may have parsed the module off disk, so cache the result.
        let fetched_result = match expression {
            // `[a, b<tab>]`: complete against the bracketed import list.
            Some(Expr::FullCellPath(full_cell_path)) => match &full_cell_path.head.expr {
                Expr::List(items) => self.complete_on_list_items(
                    items,
                    completion_context,
                    &mut exportable_completion,
                ),
                _ => Fetched::answering(vec![]),
            },
            // No expression at all or any other scalar shape (`Expr::String`, plus `Expr::Nothing`
            // for the `null` keyword): search exports by the raw prefix text.
            _ => exportable_completion.fetch(completion_context),
        };

        // Reaching here may have parsed a module off disk, so the answer is worth keeping
        // even where the source that gave it was cheap.
        fetched_result.worth_keeping()
    }

    fn find_module_name_and_span(&self) -> Option<(&[u8], Span)> {
        // Only the first positional is the module path, not later import names.
        let first = self.call.arguments.iter().find_map(|a| {
            if let Argument::Positional(e) = a {
                Some(e)
            } else {
                None
            }
        })?;

        if let Expression {
            expr: Expr::String(name),
            span,
            ..
        } = first
        {
            Some((name.as_bytes(), *span))
        } else {
            None
        }
    }

    fn resolve_module<'set>(
        &self,
        working_set: &'set StateWorkingSet,
        module_name: &[u8],
        span: Span,
    ) -> Option<(nu_protocol::ModuleId, Option<StateWorkingSet<'set>>)> {
        if let Some(module_id) = working_set.find_module(module_name) {
            return Some((module_id, None));
        }

        let mut temp_working_set = StateWorkingSet::new(working_set.permanent_state);
        let module_id = parse_module_file_or_dir(&mut temp_working_set, module_name, span, None)?;

        Some((module_id, Some(temp_working_set)))
    }

    fn complete_on_list_items(
        &self,
        items: &[ListItem],
        completion_context: &Context,
        exportable_completion: &mut ExportableCompletion,
    ) -> Fetched {
        let cursor_position = self.cursor;

        let item_span = items
            .iter()
            .map(|item| item.expr().span)
            .find(|item_span| touches(*item_span, cursor_position))
            .unwrap_or(Span::point(cursor_position));

        // The member's typed text is the tail of `completion_context.prefix`, from
        // the member's start onward; the cursor-sliced buffer already ends it.
        let relative_offset = item_span
            .start
            .saturating_sub(completion_context.span.start);
        let sliced_prefix = completion_context
            .prefix
            .get(relative_offset..)
            .unwrap_or_default();

        let new_span = Span::new(
            item_span.start,
            completion_context.span.end.min(item_span.end),
        );

        let item_context = Context {
            span: new_span,
            prefix: sliced_prefix,
            ..*completion_context
        };

        exportable_completion.fetch(&item_context)
    }

    fn fetch_fallback_completion(
        &self,
        expression: Option<&Expr>,
        completion_context: &Context,
    ) -> Fetched {
        let complete_file = || FileCompletion.fetch(completion_context);

        // Prefer the parsed argument's shape. When the slot is still empty (`cd <tab>`)
        // there is no parsed expression, so fall back to the shape the signature declares
        // for this argument; otherwise a typed positional/flag would lose its type-specific
        // completion and dump every path instead (issue #18862).
        let shape = match expression {
            Some(Expr::Directory(_, _)) => Some(&SyntaxShape::Directory),
            Some(Expr::Filepath(_, _)) => Some(&SyntaxShape::Filepath),
            Some(Expr::GlobPattern(_, _)) => Some(&SyntaxShape::GlobPattern),
            Some(_) => None,
            None => self.declared_shape.as_ref(),
        };

        match shape {
            Some(SyntaxShape::Directory) => DirectoryCompletion.fetch(completion_context),
            Some(SyntaxShape::Filepath | SyntaxShape::GlobPattern) => complete_file(),
            // fallback to file completion if necessary
            _ if self.need_fallback => complete_file(),
            _ => Fetched::answering(vec![]),
        }
    }
}
