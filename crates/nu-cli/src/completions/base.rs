use crate::completions::CompletionOptions;
use nu_color_config::NuStyle;
use nu_protocol::{
    DynamicSuggestion, IntoValue, Record, Span, SuggestionKind, Value,
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

        if let Some(kind) = self.kind.as_ref().map(|kind| match kind {
            SuggestionKind::Command(..) => "command",
            SuggestionKind::Value(_) => "value",
            SuggestionKind::CellPath => "cell-path",
            SuggestionKind::Directory => "directory",
            SuggestionKind::File => "file",
            SuggestionKind::Flag => "flag",
            SuggestionKind::Module => "module",
            SuggestionKind::Operator => "operator",
            SuggestionKind::Variable => "variable",
        }) {
            record.insert("kind", kind.into_value(span));
        }

        if let Some(ty) = self.kind.as_ref().and_then(|kind| match kind {
            SuggestionKind::Command(ty, _) => Some(ty.to_string()),
            SuggestionKind::Value(ty) => Some(ty.to_string()),
            _ => None,
        }) {
            record.insert("type", ty.into_value(span));
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
