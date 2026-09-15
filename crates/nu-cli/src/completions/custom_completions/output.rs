use super::Narrowing;
use crate::completions::{
    CompletionOptions, Context, Fetched, MatchAlgorithm, NuMatcher, SemanticSuggestion,
    to_reedline_span,
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_protocol::{FromValue, ShellError, SuggestionKind, Type, Value, engine::CommandType};
use nu_utils::strip_ansi_string_unlikely;
use reedline::Suggestion;

/// A completer's return value, after both accepted shapes are normalized.
pub(crate) struct CompleterOutput {
    suggestions: Vec<SemanticSuggestion>,
    options: CompletionOptions,
    sort: bool,
    filter: bool,
    /// Whether the completer requested the next source.
    fallback: bool,
}

/// Log a non-fatal completer error without disrupting line editing.
pub(crate) fn report(message: impl AsRef<str>) {
    log::error!("nu::shell::completion: {}", message.as_ref());
}

/// Convert a completer result field, logging conversion failures.
fn read_part<T: FromValue>(value: Value, what: &str) -> Option<T> {
    T::from_value(value)
        .map_err(|err| report(format!("a completer's {what} is not usable: {err}")))
        .ok()
}

/// A string field read leniently: anything that coerces to a string becomes one, so
/// `{description: 5}` reads as `"5"` rather than costing the suggestion it describes.
struct Text(String);

impl FromValue for Text {
    fn from_value(value: Value) -> Result<Self, ShellError> {
        value.coerce_into_string().map(Self)
    }

    fn expected_type() -> Type {
        Type::String
    }
}

impl From<Text> for String {
    fn from(text: Text) -> Self {
        text.0
    }
}

/// Warn on unknown keys in completer output.
fn lint_keys(value: &Value, what: &str, keys: &[&str]) {
    let Ok(record) = value.as_record() else {
        return;
    };

    for unknown in record
        .columns()
        .filter(|column| !keys.contains(&column.as_str()))
    {
        report(format!(
            "a {what} has no `{unknown}` field, so it is ignored; expected one of {}",
            keys.join(", ")
        ));
    }
}

/// Record returned by completer with exact accepted keys.
trait ReturnedRecord: FromValue {
    /// What to call it in a message.
    const LABEL: &'static str;
    /// Every key accepted, whether read into a field or merely tolerated.
    const KEYS: &'static [&'static str];

    /// Read one; unknown keys linted and ignored.
    fn read(value: Value) -> Option<Self> {
        lint_keys(&value, Self::LABEL, Self::KEYS);
        read_part(value, Self::LABEL)
    }
}

/// Declare a record a completer returns: the struct [`FromValue`] reads, and the exact set
/// of keys it accepts. `accepting` names keys tolerated but not read.
macro_rules! returned_record {
    (
        $(#[$doc:meta])*
        struct $name:ident is $label:literal $(, accepting [$($extra:literal),* $(,)?])? {
            $( $(#[$field_doc:meta])* $field:ident : $ty:ty ),* $(,)?
        }
    ) => {
        $(#[$doc])*
        #[derive(FromValue)]
        #[nu_value(type_name = $label)]
        struct $name {
            $( $(#[$field_doc])* $field: $ty, )*
        }

        impl ReturnedRecord for $name {
            const LABEL: &'static str = $label;
            const KEYS: &'static [&'static str] =
                &[$(stringify!($field),)* $($($extra),*)?];
        }
    };
}

returned_record! {
    /// The `options` a completer may return beside its completions. Every key is optional;
    /// one left out keeps the engine's configured setting.
    struct ReturnedOptions is "completion options" {
        /// Whether the engine narrows the list against the typed text. Defaults by completer
        /// kind; see [`Narrowing`].
        filter: Option<bool>,
        sort: Option<bool>,
        case_sensitive: Option<bool>,
        match_description: Option<bool>,
        completion_algorithm: Option<String>,
        /// Deprecated; use the substring match algorithm.
        positional: Option<bool>,
    }
}

returned_record! {
    /// A suggestion returned by a custom completer.
    struct ReturnedSuggestion is "suggestion", accepting ["type"] {
        /// Value to insert, coerced to a string.
        value: Value,
        display_override: Option<Text>,
        description: Option<Text>,
        /// Optional suggestion kind.
        kind: Option<Value>,
        /// A colour name or colour record.
        style: Option<Value>,
        /// Read on its own for the same reason as `style`.
        span: Option<Value>,
        /// Extra columns beside the value, in description menus.
        extra: Option<Vec<Text>>,
        /// Whether to append a space after the value.
        append_whitespace: Option<bool>,
        /// Character indices matching the typed text.
        match_indices: Option<Vec<usize>>,
    }
}

/// Convert a custom suggestion kind, defaulting to the value's type.
fn suggestion_kind(kind: Option<Value>, value: &Value) -> SuggestionKind {
    let by_type = || SuggestionKind::Value(value.get_type());
    let Some(kind) = kind else { return by_type() };

    let Ok(name) = kind.as_str() else {
        report(format!(
            "a suggestion's `kind` must be a name like `directory` or `command`; got {}",
            kind.get_type()
        ));
        return by_type();
    };

    match name {
        "value" => by_type(),
        "command" => SuggestionKind::Command(CommandType::Custom, None),
        "cell-path" => SuggestionKind::CellPath,
        "directory" => SuggestionKind::Directory,
        "file" => SuggestionKind::File,
        "flag" => SuggestionKind::Flag,
        "module" => SuggestionKind::Module,
        "operator" => SuggestionKind::Operator,
        "variable" => SuggestionKind::Variable,
        other => {
            report(format!(
                "a suggestion's `kind` is `{other}`; expected one of value, command, \
                 cell-path, directory, file, flag, module, operator, variable"
            ));
            by_type()
        }
    }
}

returned_record! {
    /// A `span` a suggestion returned. Each end is optional and falls back to the span the
    /// engine would have replaced, so `{end: 20}` moves only the end (#tested by
    /// `custom_completions_override_span`).
    struct ReturnedSpan is "suggestion span" {
        start: Option<i64>,
        end: Option<i64>,
    }
}

/// Keys accepted by the completion envelope.
const ENVELOPE_KEYS: [&str; 3] = ["completions", "options", "fallback"];

/// What a completer or menu source returned, before any options are applied. See the
/// [module docs](self) for the shapes accepted.
pub(crate) struct Returned {
    pub(crate) completions: Vec<Value>,
    options: Option<ReturnedOptions>,
    /// Whether to continue with the next source.
    fallback: bool,
}

impl Returned {
    /// `None` where the source declined with `null`, letting the next one answer.
    pub(crate) fn read(value: Value) -> Option<Self> {
        let bare = |completions| {
            Some(Self {
                completions,
                options: None,
                fallback: false,
            })
        };

        // Records without envelope keys are treated as lone suggestions.
        let is_envelope = match &value {
            Value::Record { val, .. } => ENVELOPE_KEYS.iter().any(|key| val.contains(key)),
            _ => false,
        };

        if is_envelope {
            lint_keys(&value, "completion envelope", &ENVELOPE_KEYS);
            let mut record = value.into_record().unwrap_or_default();

            // Read each part independently.
            return Some(Self {
                completions: record
                    .remove("completions")
                    .and_then(|completions| read_part(completions, "`completions`"))
                    .unwrap_or_default(),
                options: record.remove("options").and_then(ReturnedOptions::read),
                fallback: record
                    .remove("fallback")
                    .and_then(|fallback| read_part(fallback, "`fallback`"))
                    .unwrap_or(false),
            });
        }

        match value {
            Value::Nothing { .. } => None,
            Value::List { vals, .. } => bare(vals.into_owned()),
            // A list is many suggestions and anything else is one. A picker returns the item
            // it was given, not a list of one: `input list` and `fzf` both answer with a bare
            // string, and rejecting it would cost the completion the user just chose.
            one => bare(vec![one]),
        }
    }
}

impl CompleterOutput {
    /// Read what a completer returned; `None` when it declined with `null`.
    pub(crate) fn read(value: Value, ctx: &Context, narrowing: Narrowing) -> Option<Self> {
        let returned = Returned::read(value)?;

        let mut output = Self {
            suggestions: map_value_completions(
                returned.completions.into_iter(),
                to_reedline_span(ctx.span, ctx.offset),
                SpanClamp::within(ctx.buffer),
            ),
            options: ctx.options.clone(),
            sort: true,
            filter: narrowing.filters_by_default(),
            fallback: returned.fallback,
        };

        if let Some(options) = returned.options {
            output.read_options(options);
        }

        Some(output)
    }

    /// Apply the `options` record a completer returned alongside its completions.
    fn read_options(&mut self, options: ReturnedOptions) {
        if let Some(filter) = options.filter {
            self.filter = filter;
        }

        if let Some(sort) = options.sort {
            self.sort = sort;
            if self.sort && !self.filter {
                log::warn!("Sorting won't happen because filtering is disabled.");
            }
        }

        if let Some(case_sensitive) = options.case_sensitive {
            self.options.case_sensitive = case_sensitive;
        }

        if let Some(match_description) = options.match_description {
            self.options.match_description = match_description;
        }

        if options.positional.is_some() {
            log::warn!(
                "Use of the positional option is deprecated. Use the substring match algorithm instead."
            );
        }

        if let Some(algorithm) = options
            .completion_algorithm
            .and_then(|algorithm| MatchAlgorithm::try_from(algorithm).ok())
        {
            self.options.match_algorithm = algorithm;
        }

        if let Some(positional) = options.positional
            && !positional
            && self.options.match_algorithm == MatchAlgorithm::Prefix
        {
            self.options.match_algorithm = MatchAlgorithm::Substring;
        }
    }

    /// Narrow suggestions and preserve the completer's fallback request.
    pub(crate) fn into_fetched(self, ctx: &Context) -> Fetched {
        let fallback = self.fallback;
        let outcome = |suggestions| {
            if fallback {
                Fetched::contributing(suggestions)
            } else {
                Fetched::answering(suggestions).worth_keeping()
            }
        };

        if !self.filter {
            return outcome(self.suggestions);
        }

        let prefix = ctx.prefix_str();
        let mut matcher = NuMatcher::new(prefix.as_ref(), &self.options, self.sort);

        for suggestion in self.suggestions {
            let value =
                strip_ansi_string_unlikely(suggestion.suggestion.display_value().to_string());
            if matcher.check_match(&value).is_some() {
                matcher.add(value, suggestion);
            } else if self.options.match_description
                && let Some(description) = suggestion.suggestion.description.as_deref()
                && matcher.check_match(description).is_some()
            {
                let description = description.to_string();
                matcher.add(description, suggestion);
            }
        }

        outcome(matcher.suggestion_results())
    }
}

/// The colour a suggestion asked for: a name (`"green"`, `"bg_red"`), or a record of
/// `{fg, bg, attr}`.
fn style_of(style: Value) -> Option<nu_ansi_term::Style> {
    match style {
        Value::String { val, .. } => Some(lookup_ansi_color_style(&val)),
        Value::Record { .. } => Some(color_record_to_nustyle(&style)),
        other => {
            report(format!(
                "a suggestion's `style` must be a colour name or a record of {{fg, bg, attr}}; \
                 got {}",
                other.get_type()
            ));
            None
        }
    }
}

impl ReturnedSuggestion {
    /// Resolve against the span a suggestion replaces when it names none.
    fn into_semantic(
        self,
        default_span: reedline::Span,
        clamp: SpanClamp<'_>,
    ) -> SemanticSuggestion {
        let kind = suggestion_kind(self.kind, &self.value);

        SemanticSuggestion {
            suggestion: Suggestion {
                value: self
                    .value
                    .coerce_into_string()
                    .map(strip_ansi_string_unlikely)
                    .unwrap_or_default(),
                display_override: self.display_override.map(Into::into),
                description: self.description.map(Into::into),
                style: self.style.and_then(style_of),
                span: self
                    .span
                    .and_then(ReturnedSpan::read)
                    .map_or(default_span, |span| span.resolve(default_span, clamp)),
                extra: self
                    .extra
                    .map(|extra| extra.into_iter().map(Into::into).collect()),
                append_whitespace: self.append_whitespace.unwrap_or_default(),
                match_indices: self.match_indices,
            },
            kind: Some(kind),
        }
    }
}

impl ReturnedSpan {
    /// Clamp so a returned span can't index out of bounds or mid-character (#5127), nor
    /// replace text the completer was never shown.
    fn resolve(self, default: reedline::Span, clamp: SpanClamp<'_>) -> reedline::Span {
        let offset = |value: Option<i64>, fallback: usize| match value {
            // A negative offset names no position on the line. Keeping the default is the
            // safe reading; clamping it to `0` would silently replace from the line's start.
            Some(value) if value < 0 => {
                report(format!("a suggestion's span offset ({value}) is negative"));
                fallback
            }
            Some(value) => clamp.apply(value as usize),
            None => fallback,
        };

        let start = offset(self.start, default.start);
        let end = offset(self.end, default.end);

        if start > end {
            report(format!(
                "a suggestion's span start ({start}) is greater than its end ({end})"
            ));
            return reedline::Span::new(end, end);
        }

        reedline::Span::new(start, end)
    }
}

/// Parse one `Value` of a completer's list into a SemanticSuggestion.
pub(crate) fn map_value_completions(
    list: impl Iterator<Item = Value>,
    default_span: reedline::Span,
    clamp: SpanClamp<'_>,
) -> Vec<SemanticSuggestion> {
    list.filter_map(move |value| {
        if matches!(value, Value::Record { .. }) {
            return ReturnedSuggestion::read(value)
                .map(|suggestion| suggestion.into_semantic(default_span, clamp));
        }

        let kind = nu_protocol::SuggestionKind::Value(value.get_type());
        value
            .coerce_into_string()
            .ok()
            .map(|string| SemanticSuggestion {
                suggestion: Suggestion {
                    value: strip_ansi_string_unlikely(string),
                    span: default_span,
                    ..Suggestion::default()
                },
                kind: Some(kind),
            })
    })
    .collect()
}

/// Clamps returned spans to the text the source saw, snapping offsets to its char boundaries.
#[derive(Clone, Copy)]
pub(crate) struct SpanClamp<'a> {
    seen: &'a str,
}

impl<'a> SpanClamp<'a> {
    /// Clamped to `text`.
    pub(crate) fn within(text: &'a str) -> Self {
        Self { seen: text }
    }

    pub(crate) fn apply(&self, value: usize) -> usize {
        self.seen.floor_char_boundary(value.min(self.seen.len()))
    }
}
