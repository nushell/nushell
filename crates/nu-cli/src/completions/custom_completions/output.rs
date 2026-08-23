use super::Narrowing;
use crate::completions::{
    CompletionOptions, Context, Fetched, MatchAlgorithm, NuMatcher, SemanticSuggestion,
    to_reedline_span,
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_protocol::{FromValue, ShellError, Type, Value, shell_error::generic::GenericError};
use nu_utils::strip_ansi_string_unlikely;
use reedline::Suggestion;

/// A completer's return value, after both accepted shapes are normalized.
pub(crate) struct CompleterOutput {
    suggestions: Vec<SemanticSuggestion>,
    options: CompletionOptions,
    sort: bool,
    filter: bool,
}

/// Report something a completer returned that could not be used. Nothing here is fatal —
/// the rest of the return still stands — so the message is the only way a completer author
/// learns of it.
fn report(message: impl Into<String>) {
    log::error!(
        "{}",
        ShellError::Generic(GenericError::new_internal(
            "nu::shell::completion",
            message.into(),
        ))
    );
}

/// Read one part of what a completer returned, naming it when it does not fit. A part that
/// fails costs that part alone: a bad `options` keeps the completions returned beside it, a
/// bad `span` keeps the suggestion around it.
fn read_part<T: FromValue>(value: Value, what: &str) -> Option<T> {
    T::from_value(value)
        .map_err(|err| {
            log::error!(
                "{}",
                ShellError::Generic(
                    GenericError::new_internal(
                        "nu::shell::completion",
                        format!("a completer's {what} is not usable"),
                    )
                    .with_inner([err]),
                )
            );
        })
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

/// A record a completer returns, read strictly: only the keys it declares are accepted, so
/// a misspelled one is reported rather than silently doing nothing. Implemented solely by
/// [`returned_record!`], which derives [`Self::KEYS`] from the same fields [`FromValue`]
/// reads, so the two cannot drift apart.
trait ReturnedRecord: FromValue {
    /// What to call it in a message.
    const LABEL: &'static str;
    /// Every key accepted, whether read into a field or merely tolerated.
    const KEYS: &'static [&'static str];

    /// Read one, reporting an unknown key or a value that does not fit.
    fn read(value: Value) -> Option<Self> {
        if let Ok(record) = value.as_record()
            && let Some(unknown) = record
                .columns()
                .find(|column| !Self::KEYS.contains(&column.as_str()))
        {
            report(format!(
                "a {} has no `{unknown}` field; expected one of {}",
                Self::LABEL,
                Self::KEYS.join(", ")
            ));
            return None;
        }

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
    /// One suggestion, as every completer and menu source returns it. Declared rather than
    /// read key by key, so a mistyped field is reported, not a silent default.
    struct ReturnedSuggestion is "suggestion", accepting ["kind", "type"] {
        /// Coerced to a string to insert; its type is reported back as the suggestion's kind.
        value: Value,
        display_override: Option<Text>,
        description: Option<Text>,
        /// A colour name, or a colour record. Read on its own, so an unusable one costs the
        /// colour rather than the suggestion.
        style: Option<Value>,
        /// Read on its own for the same reason as `style`.
        span: Option<Value>,
        /// Extra columns beside the value, in description menus.
        extra: Option<Vec<Text>>,
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

/// What a completer or menu source returned, before any options are applied. See the
/// [module docs](self) for the shapes accepted.
pub(crate) struct Returned {
    pub(crate) completions: Vec<Value>,
    options: Option<ReturnedOptions>,
}

impl Returned {
    /// `None` where the source declined with `null`, letting the next one answer.
    pub(crate) fn read(value: Value) -> Option<Self> {
        let bare = |completions| {
            Some(Self {
                completions,
                options: None,
            })
        };

        // A record is the envelope only when it names one of the two keys; any other record
        // is a lone suggestion, which is what a menu source returning one has always meant.
        let is_envelope = match &value {
            Value::Record { val, .. } => val.contains("completions") || val.contains("options"),
            _ => false,
        };

        if is_envelope {
            let mut record = value.into_record().unwrap_or_default();

            // Check for unknown keys
            let mut unknown_keys = Vec::new();
            for col in record.columns() {
                if col != "completions" && col != "options" {
                    unknown_keys.push(col.clone());
                }
            }
            if !unknown_keys.is_empty() {
                report(format!(
                    "a completion envelope has unknown fields: {}; expected `completions` or `options`",
                    unknown_keys.join(", ")
                ));
            }

            // Read the halves apart, so a bad `options` costs the settings it names and not
            // every completion returned beside it.
            return Some(Self {
                completions: record
                    .remove("completions")
                    .and_then(|completions| read_part(completions, "`completions`"))
                    .unwrap_or_default(),
                options: record.remove("options").and_then(ReturnedOptions::read),
            });
        }

        match value {
            Value::Nothing { .. } => None,
            Value::List { vals, .. } => bare(vals.into_owned()),
            record @ Value::Record { .. } => bare(vec![record]),
            // A list is many suggestions and a record is one, so a bare value is neither —
            // a mistake, not a shorthand. Its elements would have coerced inside a list.
            other => {
                report(format!(
                    "a completer must return a list of completions, a record of \
                     {{completions, options?}} or one suggestion, or null to decline; got {}",
                    other.get_type()
                ));
                bare(Vec::new())
            }
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

    /// Narrow the suggestions against the typed prefix, unless the completer already did.
    pub(crate) fn into_fetched(self, ctx: &Context) -> Fetched {
        if !self.filter {
            return Fetched::Cacheable(self.suggestions);
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

        Fetched::Cacheable(matcher.suggestion_results())
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
        let kind = nu_protocol::SuggestionKind::Value(self.value.get_type());

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
                ..Suggestion::default()
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

/// How far a returned span may reach, and against which text. Completers clamp to the
/// buffer they saw; menu sources to the cursor, with no text to snap to a char boundary.
#[derive(Clone, Copy)]
pub(crate) struct SpanClamp<'a> {
    pub limit: usize,
    pub text: Option<&'a str>,
}

impl<'a> SpanClamp<'a> {
    /// Clamped to `text`, whose length is the bound.
    pub(crate) fn within(text: &'a str) -> Self {
        Self {
            limit: text.len(),
            text: Some(text),
        }
    }

    /// Bounded by an offset with no text to align against.
    pub(crate) fn upto(limit: usize) -> Self {
        Self { limit, text: None }
    }

    pub(crate) fn apply(&self, value: usize) -> usize {
        let value = value.min(self.limit);
        self.text.map_or(value, |text| {
            text.floor_char_boundary(value.min(text.len()))
        })
    }
}
