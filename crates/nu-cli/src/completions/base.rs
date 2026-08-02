use crate::completions::completer::Context;
use nu_color_config::NuStyle;
use nu_protocol::{DynamicSuggestion, IntoValue, Record, Span, SuggestionKind, Value};
use reedline::Suggestion;

pub trait Completer {
    /// Fetch, filter, and sort completions for the token described by `ctx`.
    fn fetch(&mut self, ctx: &Context) -> Fetched;
}

/// The outcome of one source's [`Completer::fetch`], plus two flags the machinery cannot
/// infer from the suggestions: `cacheable` (impure sources worth reusing between keystrokes)
/// and `need_fallback` (try the next source). Fields are private, so a declining result can
/// never also carry suggestions.
#[derive(Debug, Default)]
pub struct Fetched {
    pub(crate) suggestions: Vec<SemanticSuggestion>,
    pub(crate) cacheable: bool,
    pub(crate) need_fallback: bool,
}

impl Fetched {
    /// A cheap engine-state result: never cached, never falls back.
    pub(crate) fn pure(suggestions: Vec<SemanticSuggestion>) -> Self {
        Self {
            suggestions,
            cacheable: false,
            need_fallback: false,
        }
    }

    /// An impure source's result (filesystem, `PATH`, user/plugin code); worth caching.
    pub(crate) fn cacheable(suggestions: Vec<SemanticSuggestion>) -> Self {
        Self {
            suggestions,
            cacheable: true,
            need_fallback: false,
        }
    }

    /// An impure source that declined: fall back, but stay `cacheable` since the
    /// expensive attempt ran.
    pub(crate) fn fallback() -> Self {
        Self {
            suggestions: vec![],
            cacheable: true,
            need_fallback: true,
        }
    }

    /// Like [`Self::fallback`] but cheap — no source ran, so nothing to cache.
    pub(crate) fn absent() -> Self {
        Self {
            suggestions: vec![],
            cacheable: false,
            need_fallback: true,
        }
    }

    /// Force [`Self::cacheable`] on when the caller did expensive work (e.g. parsing a
    /// module off disk) around a cheap lookup.
    pub(crate) fn caching(mut self) -> Self {
        self.cacheable = true;
        self
    }
}

/// Convert an engine [`Span`] to reedline coordinates by subtracting the working-set
/// `offset`. Both ends saturate so spans before `offset` can't underflow into an index that
/// would panic (`is_char_boundary`); callers may pass untrusted spans.
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

            if let Some(ty) = ty {
                record.insert("type", ty.into_value(span));
            }
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
