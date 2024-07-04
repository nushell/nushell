use crate::completions::CompletionOptions;
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span,
};
use reedline::Suggestion;

pub trait Completer {
    /// Fetch, filter, and sort completions
    #[allow(clippy::too_many_arguments)]
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: Vec<u8>,
        span: Span,
        offset: usize,
        pos: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion>;
}

#[derive(Debug, Default, PartialEq)]
pub struct SemanticSuggestion {
    pub suggestion: Suggestion,
    pub kind: Option<SuggestionKind>,
}

// TODO: think about name: maybe suggestion context?
#[derive(Clone, Debug, PartialEq)]
pub enum SuggestionKind {
    Command(nu_protocol::engine::CommandType),
    Type(nu_protocol::Type),
}

impl From<Suggestion> for SemanticSuggestion {
    fn from(suggestion: Suggestion) -> Self {
        Self {
            suggestion,
            ..Default::default()
        }
    }
}
