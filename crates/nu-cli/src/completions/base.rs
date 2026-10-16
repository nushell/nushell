use crate::completions::completer::Context;
use nu_color_config::NuStyle;
use nu_protocol::{DynamicSuggestion, IntoValue, Record, Span, SuggestionKind, Value};
use reedline::Suggestion;

pub trait Completer {
    /// Fetch, filter, and sort completions for the token described by `ctx`.
    fn fetch(&mut self, ctx: &Context) -> Fetched;
}

/// Result from one completion source.
#[derive(Debug, Default)]
pub struct Fetched {
    suggestions: Vec<SemanticSuggestion>,
    /// Source answered; no fallback.
    answered: bool,
    /// Worth caching between keystrokes.
    reusable: bool,
}

impl Fetched {
    /// No source ran here.
    pub(crate) fn absent() -> Self {
        Self::default()
    }

    /// Answer with `suggestions`; empty still counts as answered.
    pub(crate) fn answering(suggestions: Vec<SemanticSuggestion>) -> Self {
        Self {
            suggestions,
            answered: true,
            reusable: false,
        }
    }

    /// Contribute suggestions and allow fallback.
    pub(crate) fn contributing(suggestions: Vec<SemanticSuggestion>) -> Self {
        Self {
            suggestions,
            answered: false,
            reusable: true,
        }
    }

    /// Decline; next source answers.
    pub(crate) fn declining() -> Self {
        Self {
            suggestions: Vec::new(),
            answered: false,
            reusable: true,
        }
    }

    /// Mark answer cacheable.
    pub(crate) fn worth_keeping(mut self) -> Self {
        self.reusable = true;
        self
    }

    /// The suggestions this outcome carries.
    pub(crate) fn into_suggestions(self) -> Vec<SemanticSuggestion> {
        self.suggestions
    }

    /// Did source answer?
    pub(crate) fn answered(&self) -> bool {
        self.answered
    }

    /// Cacheable between keystrokes?
    pub(crate) fn is_reusable(&self) -> bool {
        self.reusable
    }

    /// Any suggestions yet, without consuming them.
    pub(crate) fn is_empty(&self) -> bool {
        self.suggestions.is_empty()
    }

    /// Keep only matching suggestions (the shorter-head reading drops commands).
    pub(crate) fn retain(&mut self, f: impl FnMut(&SemanticSuggestion) -> bool) {
        self.suggestions.retain(f);
    }

    /// Append another outcome's suggestions and state: answered only if both
    /// answered, reusable if either is.
    pub(crate) fn merge(&mut self, other: Fetched) {
        self.answered |= other.answered;
        self.reusable |= other.reusable;
        self.suggestions.extend(other.suggestions);
    }

    /// Merge one source's outcome and report whether it answered.
    pub(crate) fn absorb(&mut self, attempt: Fetched) -> bool {
        let answered = attempt.answered();
        self.merge(attempt);
        answered
    }

    /// Prepend another outcome's suggestions, sharing reusability but leaving
    /// answeredness untouched (for the shorter-head argument reading, whose
    /// ranking comes first but which never settles the site).
    pub(crate) fn prepend_from(&mut self, other: Fetched) {
        self.reusable |= other.reusable;
        let mut combined = other.suggestions;
        combined.append(&mut self.suggestions);
        self.suggestions = combined;
    }
}

/// An engine [`Span`] in reedline coordinates: subtract `offset`, saturating so spans
/// before it can't underflow into an index that would panic (`is_char_boundary`); callers
/// may pass untrusted spans.
pub(crate) fn to_reedline_span(span: Span, offset: usize) -> reedline::Span {
    reedline::Span::new(
        span.start.saturating_sub(offset),
        span.end.saturating_sub(offset),
    )
}

#[derive(Debug, Default, PartialEq)]
pub struct SemanticSuggestion {
    pub suggestion: Suggestion,
    pub kind: Option<SuggestionKind>,
}

impl SemanticSuggestion {
    pub fn from_dynamic_suggestion(
        suggestion: DynamicSuggestion,
        span: reedline::Span,
        style: Option<nu_ansi_term::Style>,
    ) -> Self {
        SemanticSuggestion {
            suggestion: Suggestion {
                value: suggestion.value,
                display_override: suggestion.display_override,
                description: suggestion.description,
                extra: suggestion.extra,
                append_whitespace: suggestion.append_whitespace,
                match_indices: suggestion.match_indices,
                style,
                span,
            },
            kind: suggestion.kind,
        }
    }
}

impl IntoValue for SemanticSuggestion {
    fn into_value(self, span: Span) -> Value {
        let mut record = Record::new();
        record.insert("value", Value::string(self.suggestion.value, span));

        if let Some(span_rec) = span_record(self.suggestion.span, span) {
            record.insert("span", span_rec);
        }

        if let Some(display) = self.suggestion.display_override {
            record.insert("display_override", Value::string(display, span));
        }

        if let Some(style) = self.suggestion.style.map(NuStyle::from) {
            record.insert("style", style.into_value(span));
        }

        if let Some(description) = self.suggestion.description {
            record.insert("description", description.into_value(span));
        }

        if let Some(extra) = self.suggestion.extra {
            record.insert("extra", extra.into_value(span));
        }

        // Omit default fields to keep the common output compact.
        if self.suggestion.append_whitespace {
            record.insert("append_whitespace", Value::bool(true, span));
        }

        if let Some(match_indices) = self
            .suggestion
            .match_indices
            .filter(|indices| !indices.is_empty())
        {
            record.insert(
                "match_indices",
                Value::list(
                    match_indices
                        .into_iter()
                        .map(|index| Value::int(index as i64, span))
                        .collect(),
                    span,
                ),
            );
        }

        if let Some(kind) = self.kind {
            let (kind_str, ty) = match kind {
                SuggestionKind::Command(ty, _) => ("command", Some(ty.to_string())),
                SuggestionKind::Value(ty) => ("value", Some(ty.to_string())),
                SuggestionKind::CellPath => ("cell-path", None),
                SuggestionKind::Directory => ("directory", None),
                SuggestionKind::File => ("file", None),
                SuggestionKind::Flag => ("flag", None),
                SuggestionKind::Module => ("module", None),
                SuggestionKind::Operator => ("operator", None),
                SuggestionKind::Variable => ("variable", None),
            };
            record.insert("kind", kind_str.into_value(span));

            // Always a column: kinds without a type report `null`.
            record.insert(
                "type",
                ty.map_or_else(|| Value::nothing(span), |ty| ty.into_value(span)),
            );
        }

        Value::record(record, span)
    }
}

fn span_record(span: reedline::Span, src_span: Span) -> Option<Value> {
    let (Ok(start), Ok(end)) = (span.start.try_into(), span.end.try_into()) else {
        log::error!("failed to convert span to i64s");
        return None;
    };

    Some(Value::record(
        Record::from_iter([
            ("start".into(), Value::int(start, src_span)),
            ("end".into(), Value::int(end, src_span)),
        ]),
        src_span,
    ))
}

impl From<Suggestion> for SemanticSuggestion {
    fn from(suggestion: Suggestion) -> Self {
        Self {
            suggestion,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declining_carries_no_suggestions() {
        assert!(!Fetched::declining().answered());
        assert!(Fetched::declining().into_suggestions().is_empty());
        assert!(!Fetched::absent().answered());
    }

    #[test]
    fn answering_with_nothing_still_answers() {
        let dismissed = Fetched::answering(vec![]);
        assert!(dismissed.answered());
        assert!(!dismissed.is_reusable());
        assert!(dismissed.into_suggestions().is_empty());
    }

    #[test]
    fn contributing_keeps_its_suggestions_and_the_site_open() {
        let fetched = Fetched::contributing(vec![SemanticSuggestion::default()]);
        assert!(!fetched.answered());
        assert_eq!(fetched.into_suggestions().len(), 1);
    }
}
