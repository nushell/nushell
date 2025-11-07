use crate::completions::CompletionOptions;
use nu_protocol::{
    DeclId, DynamicSemanticSuggestion, DynamicSuggestion, Span, SuggestionKind,
    engine::{Stack, StateWorkingSet},
};
use reedline::Suggestion;

pub trait Completer {
    /// Fetch, filter, and sort completions
    #[allow(clippy::too_many_arguments)]
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion>;
}

#[derive(Debug, Default, PartialEq)]
pub struct SemanticSuggestion {
    pub suggestion: Suggestion,
    pub kind: Option<SuggestionKind>,
}

impl SemanticSuggestion {
    pub fn from_dynamic_suggestion(
        suggestion: DynamicSemanticSuggestion,
        span: Span,
        style: Option<nu_ansi_term::Style>,
    ) -> Self {
        SemanticSuggestion {
            suggestion: Suggestion {
                value: suggestion.suggestion.value,
                description: suggestion.suggestion.description,
                extra: suggestion.suggestion.extra,
                append_whitespace: suggestion.suggestion.append_whitespace,
                match_indices: suggestion.suggestion.match_indices,
                style,
                span,
            },
            kind: suggestion.kind,
        }
    }
}

impl From<Suggestion> for SemanticSuggestion {
    fn from(suggestion: Suggestion) -> Self {
        Self {
            suggestion,
            ..Default::default()
        }
    }
}
