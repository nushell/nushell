use crate::completions::{Completer, CompletionOptions, SemanticSuggestion};
use nu_protocol::{
    engine::{Stack, StateWorkingSet},
    Span,
};

use super::{completer::map_string_completions, completion_options::NuMatcher};

pub struct OptionsCompletion {
    options: Vec<String>,
}

impl OptionsCompletion {
    pub fn new(options: Vec<String>) -> Self {
        Self { options }
    }
}

impl Completer for OptionsCompletion {
    fn fetch(
        &mut self,
        _working_set: &StateWorkingSet,
        _stack: &Stack,
        prefix: &[u8],
        span: Span,
        offset: usize,
        _pos: usize,
        orig_options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        let completion_options = orig_options.clone();

        // Parse result
        let suggestions =
            map_string_completions(self.options.iter().map(String::as_str), span, offset);

        let mut matcher = NuMatcher::new(String::from_utf8_lossy(prefix), completion_options);

        for sugg in suggestions {
            matcher.add_semantic_suggestion(sugg);
        }
        matcher.results()
    }
}
