use crate::{
    FileCompletion, NuCompleter,
    completions::{
        CommandCompletion, Completer, CompletionOptions, DirectoryCompletion, DotNuCompletion,
        ExportableCompletion, SemanticSuggestion, completer::Context,
        completion_options::NuMatcher,
    },
};
use nu_parser::parse_module_file_or_dir;
use nu_protocol::{
    Span,
    ast::{Argument, Call, Expr, Expression, ListItem},
    engine::{ArgType, Stack, StateWorkingSet},
};

pub struct ArgValueCompletion<'a> {
    pub call: &'a Call,
    pub arg_type: ArgType<'a>,
    pub need_fallback: bool,
    pub completer: &'a NuCompleter,
    pub arg_idx: usize,
    pub pos: usize,
}

impl<'a> Completer for ArgValueCompletion<'a> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        // if user input `--foo abc`, then the `prefix` here is abc.
        let mut matcher = NuMatcher::new(prefix.as_ref(), options, true);

        let decl = working_set.get_decl(self.call.decl_id);
        let mut stack = stack.to_owned();

        match decl.get_dynamic_completion(working_set.permanent_state, &mut stack, &self.arg_type) {
            Ok(Some(items)) => {
                for i in items {
                    let suggestion = SemanticSuggestion::from_dynamic_suggestion(
                        i,
                        reedline::Span {
                            start: span.start - offset,
                            end: span.end - offset,
                        },
                        None,
                    );
                    matcher.add_semantic_suggestion(suggestion);
                }
                return matcher.suggestion_results();
            }
            Err(e) => {
                log::error!(
                    "error on fetching dynamic suggestion on {} with {:?}: {e}",
                    decl.name(),
                    self.arg_type
                );
            }
            // fallback to type based completion, file completion, etc.
            Ok(None) => (),
        }

        let command_head = decl.name();
        let ctx = Context::new(working_set, span, prefix.as_ref().as_bytes(), offset);
        let expr = self
            .call
            .arguments
            .get(self.arg_idx)
            .expect("Argument index out of range")
            .expr()
            .map(|e| &e.expr);

        // TODO: Move command specific completion logic to its `get_dynamic_completion`
        if let ArgType::Positional(positional_arg_index) = self.arg_type {
            match command_head {
                // complete module file/directory
                "use" | "export use" | "overlay use" | "source-env"
                    if positional_arg_index == 0 =>
                {
                    return self.completer.process_completion(
                        &mut DotNuCompletion {
                            std_virtual_path: command_head != "source-env",
                        },
                        &ctx,
                    );
                }
                // NOTE: if module file already specified,
                // should parse it to get modules/commands/consts to complete
                "use" | "export use" => {
                    let Some((module_name, span)) = self.call.arguments.iter().find_map(|arg| {
                        if let Argument::Positional(Expression {
                            expr: Expr::String(module_name),
                            span,
                            ..
                        }) = arg
                        {
                            Some((module_name.as_bytes(), span))
                        } else {
                            None
                        }
                    }) else {
                        return vec![];
                    };

                    let (module_id, temp_working_set) = match working_set.find_module(module_name) {
                        Some(module_id) => (module_id, None),
                        None => {
                            let mut temp_working_set =
                                StateWorkingSet::new(working_set.permanent_state);
                            let Some(module_id) = parse_module_file_or_dir(
                                &mut temp_working_set,
                                module_name,
                                *span,
                                None,
                            ) else {
                                return vec![];
                            };
                            (module_id, Some(temp_working_set))
                        }
                    };
                    let mut exportable_completion = ExportableCompletion {
                        module_id,
                        temp_working_set,
                    };
                    let mut complete_on_list_items =
                        |items: &[ListItem]| -> Vec<SemanticSuggestion> {
                            for item in items {
                                let span = item.expr().span;
                                if span.contains(self.pos) {
                                    let offset = span.start.saturating_sub(ctx.span.start);
                                    let end_offset = ctx
                                        .prefix
                                        .len()
                                        .min(self.pos.min(span.end) - ctx.span.start + 1);
                                    let new_ctx = Context::new(
                                        ctx.working_set,
                                        Span::new(span.start, ctx.span.end.min(span.end)),
                                        ctx.prefix.get(offset..end_offset).unwrap_or_default(),
                                        ctx.offset,
                                    );
                                    return self
                                        .completer
                                        .process_completion(&mut exportable_completion, &new_ctx);
                                }
                            }
                            vec![]
                        };

                    return match expr {
                        Some(Expr::String(_)) => self
                            .completer
                            .process_completion(&mut exportable_completion, &ctx),
                        Some(Expr::FullCellPath(fcp)) => match &fcp.head.expr {
                            Expr::List(items) => complete_on_list_items(items),
                            _ => vec![],
                        },
                        _ => vec![],
                    };
                }
                "which" => {
                    let mut completer = CommandCompletion {
                        internals: true,
                        externals: true,
                    };
                    return self.completer.process_completion(&mut completer, &ctx);
                }
                "attr complete" => {
                    let mut completer = CommandCompletion {
                        internals: true,
                        externals: false,
                    };
                    return self.completer.process_completion(&mut completer, &ctx);
                }
                _ => (),
            }
        };

        // general positional arguments
        let file_completion_helper =
            || self.completer.process_completion(&mut FileCompletion, &ctx);
        match expr {
            Some(Expr::Directory(_, _)) => self
                .completer
                .process_completion(&mut DirectoryCompletion, &ctx),
            Some(Expr::Filepath(_, _)) | Some(Expr::GlobPattern(_, _)) => file_completion_helper(),
            // fallback to file completion if necessary
            _ if self.need_fallback => file_completion_helper(),
            _ => vec![],
        }
    }
}
