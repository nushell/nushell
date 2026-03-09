use nu_protocol::NuMatcher;

use super::SemanticSuggestion;

pub(crate) fn add_semantic_suggestion(
    matcher: &mut NuMatcher<'_, SemanticSuggestion>,
    sugg: SemanticSuggestion,
) -> bool {
    let value = sugg.suggestion.display_value().to_string();
    matcher.add(value, sugg)
}

/// Get all the items that matched (sorted)
pub(crate) fn suggestion_results(
    matcher: NuMatcher<SemanticSuggestion>,
) -> Vec<SemanticSuggestion> {
    matcher
        .results()
        .into_iter()
        .map(|(mut sugg, indices)| {
            sugg.suggestion.match_indices = Some(indices);
            sugg
        })
        .collect()
}
