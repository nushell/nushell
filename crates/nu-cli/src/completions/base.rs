use crate::completions::completer::Context;
use nu_color_config::NuStyle;
use nu_protocol::{DynamicSuggestion, IntoValue, Record, Span, SuggestionKind, Value};
use reedline::Suggestion;

pub trait Completer {
    /// Fetch, filter, and sort completions for the token described by `ctx`.
    fn fetch(&mut self, ctx: &Context) -> Fetched;
}

/// The outcome of one source's [`Completer::fetch`]. Caching and fallback are encoded in
/// the variant, so a declining result cannot carry suggestions.
#[derive(Debug, Default)]
pub enum Fetched {
    /// Cheap engine-state result: never cached, never falls back.
    Pure(Vec<SemanticSuggestion>),
    /// Impure source result (filesystem, `PATH`, user/plugin code); worth caching.
    Cacheable(Vec<SemanticSuggestion>),
    /// Impure source declined: fall back, but cache the attempt.
    Declined,
    /// No source ran: fall back cheaply.
    #[default]
    Absent,
}

impl Fetched {
    /// The suggestions this outcome carries; declining outcomes carry none.
    pub(crate) fn into_suggestions(self) -> Vec<SemanticSuggestion> {
        match self {
            Self::Pure(suggestions) | Self::Cacheable(suggestions) => suggestions,
            Self::Declined | Self::Absent => Vec::new(),
        }
    }

    /// Impure source ran; result worth reusing between keystrokes.
    pub(crate) fn is_cacheable(&self) -> bool {
        matches!(self, Self::Cacheable(_) | Self::Declined)
    }

    /// Whether this source declined, so the next one should be tried.
    pub(crate) fn needs_fallback(&self) -> bool {
        matches!(self, Self::Declined | Self::Absent)
    }

    /// Mark cheap results cacheable when the caller did expensive work.
    pub(crate) fn caching(self) -> Self {
        match self {
            Self::Pure(suggestions) => Self::Cacheable(suggestions),
            Self::Absent => Self::Declined,
            already => already,
        }
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

#[derive(Clone, Debug, Default, PartialEq)]
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

    /// `complete_argument_value` relies on this to treat `need_fallback`-requesting
    /// outcomes as always empty (a dead check was removed on that assumption).
    #[test]
    fn fallback_variants_carry_no_suggestions() {
        assert!(Fetched::Declined.into_suggestions().is_empty());
        assert!(Fetched::Absent.into_suggestions().is_empty());
    }
}
