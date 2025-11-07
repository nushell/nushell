use crate::DeclId;
use crate::Type;
use crate::engine::CommandType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize, Clone)]
pub struct DynamicSemanticSuggestion {
    pub suggestion: DynamicSuggestion,
    pub kind: Option<SuggestionKind>,
}

impl From<String> for DynamicSemanticSuggestion {
    fn from(value: String) -> Self {
        Self {
            suggestion: DynamicSuggestion {
                value,
                append_whitespace: true,
                ..DynamicSuggestion::default()
            },
            kind: None,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicSuggestion {
    /// String replacement that will be introduced to the the buffer
    pub value: String,
    /// Optional description for the replacement
    pub description: Option<String>,
    /// Optional vector of strings in the suggestion. These can be used to
    /// represent examples coming from a suggestion
    pub extra: Option<Vec<String>>,
    /// Whether to append a space after selecting this suggestion.
    /// This helps to avoid that a completer repeats the complete suggestion.
    pub append_whitespace: bool,
    /// Indices of the graphemes in the suggestion that matched the typed text.
    /// Useful if using fuzzy matching.
    pub match_indices: Option<Vec<usize>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SuggestionKind {
    Command(CommandType, Option<DeclId>),
    Value(Type),
    CellPath,
    Directory,
    File,
    Flag,
    Module,
    Operator,
    Variable,
}

impl From<DynamicSuggestion> for DynamicSemanticSuggestion {
    fn from(suggestion: DynamicSuggestion) -> Self {
        Self {
            suggestion,
            ..Default::default()
        }
    }
}
