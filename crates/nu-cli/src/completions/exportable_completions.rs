use crate::completions::{
    completion_common::surround_remove, completion_options::NuMatcher, Completer,
    CompletionOptions, SemanticSuggestion, SuggestionKind,
};
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    ModuleId, Span,
};
use reedline::Suggestion;

pub struct ExportableCompletion<'a> {
    pub module_id: ModuleId,
    pub temp_working_set: Option<StateWorkingSet<'a>>,
}

/// If name contains space, wrap it in quotes
fn wrapped_name(name: String) -> String {
    if !name.contains(' ') {
        return name;
    }
    if name.contains('\'') {
        format!("\"{}\"", name.replace('"', r#"\""#))
    } else {
        format!("'{name}'")
    }
}

impl Completer for ExportableCompletion<'_> {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let mut matcher = NuMatcher::<()>::new(surround_remove(prefix.as_ref()), options);
        let mut results = Vec::new();
        let span = reedline::Span {
            start: span.start - offset,
            end: span.end - offset,
        };
        // TODO: use matcher.add_lazy to lazy evaluate an item if it matches the prefix
        let mut add_suggestion = |value: String,
                                  description: Option<String>,
                                  extra: Option<Vec<String>>,
                                  kind: SuggestionKind| {
            results.push(SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    span,
                    description,
                    extra,
                    ..Suggestion::default()
                },
                kind: Some(kind),
            });
        };

        let working_set = self.temp_working_set.as_ref().unwrap_or(working_set);
        let module = working_set.get_module(self.module_id);

        for (name, decl_id) in &module.decls {
            let name = String::from_utf8_lossy(name).to_string();
            if matcher.matches(&name) {
                let cmd = working_set.get_decl(*decl_id);
                add_suggestion(
                    wrapped_name(name),
                    Some(cmd.description().to_string()),
                    None,
                    SuggestionKind::Command(cmd.command_type()),
                );
            }
        }
        for (name, module_id) in &module.submodules {
            let name = String::from_utf8_lossy(name).to_string();
            if matcher.matches(&name) {
                let comments = working_set.get_module_comments(*module_id).map(|spans| {
                    spans
                        .iter()
                        .map(|sp| {
                            String::from_utf8_lossy(working_set.get_span_contents(*sp)).into()
                        })
                        .collect::<Vec<String>>()
                });
                add_suggestion(
                    wrapped_name(name),
                    Some("Submodule".into()),
                    comments,
                    SuggestionKind::Module,
                );
            }
        }
        for (name, var_id) in &module.constants {
            let name = String::from_utf8_lossy(name).to_string();
            if matcher.matches(&name) {
                let var = working_set.get_variable(*var_id);
                add_suggestion(
                    wrapped_name(name),
                    var.const_val
                        .as_ref()
                        .and_then(|v| v.clone().coerce_into_string().ok()),
                    None,
                    SuggestionKind::Variable,
                );
            }
        }
        results
    }
}
