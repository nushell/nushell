use crate::{DeclId, Span, Type, ast, engine::CommandType};
use serde::{Deserialize, Serialize};

/// A simple semantics suggestion just like nu_cli::SemanticSuggestion, but it
/// derives `Serialize` and `Deserialize`, so plugins are allowed to use it
/// to provide dynamic completion items.
///
/// Why define a new one rather than put `nu_cli::SemanticSuggestion` here?
///
/// If bringing `nu_cli::SemanticSuggestion` here, it brings reedline::Suggestion too,
/// then it requires this crates depends on `reedline`, this is not good because
/// protocol should not rely on a cli relative interface.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct DynamicSuggestion {
    /// String replacement that will be introduced to the the buffer
    pub value: String,
    /// If given, overrides `value` as text displayed to user
    pub display_override: Option<String>,
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
    /// Replacement span in the buffer, if any.
    pub span: Option<Span>,
    pub kind: Option<SuggestionKind>,
}

impl Default for DynamicSuggestion {
    fn default() -> Self {
        Self {
            append_whitespace: true,
            value: String::new(),
            display_override: None,
            description: None,
            extra: None,
            match_indices: None,
            kind: None,
            span: None,
        }
    }
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

/// A simple wrapper for [`ast::Call`] which contains additional context about completion.
/// It's used only at nushell side, to avoid unnecessary clone.
#[derive(Clone, Debug, PartialEq)]
pub struct DynamicCompletionCallRef<'a> {
    /// the real call, which is generated during parse time.
    pub call: &'a ast::Call,
    /// Indicates if there is a placeholder in input buffer.
    pub strip: bool,
    /// The position in input buffer, which is useful to find placeholder from arguments.
    pub pos: usize,
}
