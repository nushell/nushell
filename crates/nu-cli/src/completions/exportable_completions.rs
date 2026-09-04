use crate::completions::{
    Completer, Context, Fetched, SemanticSuggestion, completion_common::surround_remove,
    completion_options::NuMatcher, to_reedline_span,
};
use nu_protocol::{ModuleId, SuggestionKind, engine::StateWorkingSet};
use reedline::Suggestion;

pub struct ExportableCompletion<'a> {
    pub module_id: ModuleId,
    pub temp_working_set: Option<StateWorkingSet<'a>>,
}

/// If name contains space, wrap it in quotes
fn wrapped_name(name: String) -> String {
    if nu_utils::needs_quoting(&name) {
        nu_utils::escape_quote_string(&name)
    } else {
        name
    }
}

impl Completer for ExportableCompletion<'_> {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let working_set = ctx.working_set;
        let prefix = ctx.prefix_str();
        let mut matcher = NuMatcher::<SemanticSuggestion>::new(
            surround_remove(prefix.as_ref()),
            ctx.options,
            false,
        );
        let span = to_reedline_span(ctx.span, ctx.offset);
        // TODO: use matcher.add_lazy to lazy evaluate an item if it matches the prefix
        let make_suggestion =
            |value: String,
             description: Option<String>,
             extra: Option<Vec<String>>,
             kind: SuggestionKind| SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    span,
                    description,
                    extra,
                    match_indices: None,
                    ..Suggestion::default()
                },
                kind: Some(kind),
            };

        let working_set = self.temp_working_set.as_ref().unwrap_or(working_set);
        let module = working_set.get_module(self.module_id);

        for (name, decl_id) in &module.decls {
            let name = String::from_utf8_lossy(name).into_owned();
            if matcher.check_match(&name).is_none() {
                continue;
            }

            let cmd = working_set.get_decl(*decl_id);
            matcher.add_semantic_suggestion(make_suggestion(
                wrapped_name(name),
                Some(cmd.description().to_string()),
                None,
                // `None` here avoids arguments being expanded by snippet edit style for lsp
                SuggestionKind::Command(cmd.command_type(), None),
            ));
        }
        for (name, module_id) in &module.submodules {
            let name = String::from_utf8_lossy(name).into_owned();
            if matcher.check_match(&name).is_none() {
                continue;
            }

            let (desc, extra) = working_set
                .get_module_comments(*module_id)
                .map(|spans| working_set.build_desc(spans))
                .unzip();
            matcher.add_semantic_suggestion(make_suggestion(
                wrapped_name(name),
                desc.or_else(|| Some("Submodule".into())),
                extra.map(|s| vec![s]),
                SuggestionKind::Module,
            ));
        }
        for (name, var_id) in &module.constants {
            let name = String::from_utf8_lossy(name).into_owned();
            if matcher.check_match(&name).is_none() {
                continue;
            }

            let var = working_set.get_variable(*var_id);
            matcher.add_semantic_suggestion(make_suggestion(
                wrapped_name(name),
                var.const_val
                    .as_ref()
                    .and_then(|v| v.clone().coerce_into_string().ok()),
                None,
                SuggestionKind::Variable,
            ));
        }
        Fetched::Pure(matcher.suggestion_results())
    }
}
