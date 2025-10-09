use crate::completions::{
    Completer, CompletionOptions,
    completion_common::{AdjustView, adjust_if_intermediate, complete_item},
};
use nu_protocol::{
    Span,
    engine::{EngineState, Stack, StateWorkingSet},
};
use reedline::Suggestion;
use std::path::Path;

use super::{SemanticSuggestion, SuggestionKind, completion_common::FileSuggestion};

pub struct FlagValueCompletion;
impl Completer for FlagValueCompletion {
    fn fetch(
        &mut self,
        working_set: &StateWorkingSet,
        stack: &Stack,
        prefix: impl AsRef<str>,
        span: Span,
        offset: usize,
        options: &CompletionOptions,
    ) -> Vec<SemanticSuggestion> {
        vec![]
    }
}
