//! The unified record handed to user completers: `{site, tokens, context_start, cursor}`.
//! Positions are byte offsets into the commandline; a completer never sees text past the
//! cursor. `tokens.0` is the command name, the last token is under the cursor, and its
//! `span` is what a suggestion replaces. An alias-produced token has a null `span`: it is
//! not on the line.
//!
//! [`menu_input`] is the parse-free variant for menu sources.

use crate::completions::{
    Completer, CompletionOptions, Context, Fetched, MatchAlgorithm, NuMatcher, SemanticSuggestion,
    completer::is_flag_text, to_reedline_span,
};
use nu_color_config::{color_record_to_nustyle, lookup_ansi_color_style};
use nu_engine::compile;
use nu_parser::{FlatShape, flatten_expression};
use nu_protocol::{
    BlockId, DeclId, IntoValue, PipelineData, Record, ShellError, Span, SuggestionKind, Type,
    Value, VarId,
    debugger::WithoutDebug,
    engine::{Closure, Command, EngineState, StateWorkingSet},
    shell_error::generic::GenericError,
};
use nu_utils::{SharedCow, strip_ansi_string_unlikely};
use reedline::Suggestion;
use std::{borrow::Cow, sync::Arc};

/// Who filters the candidates against the typed prefix; overridable via `options.filter`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Narrowing {
    /// The engine filters; parameter completers list every candidate.
    Engine,
    /// The completer narrowed its own list; command-wide/external completers see the typed
    /// text and may match fuzzily (e.g. carapace).
    Completer,
}

impl Narrowing {
    /// Whether the engine filters when the completer expresses no `options.filter`.
    fn filters_by_default(self) -> bool {
        matches!(self, Self::Engine)
    }
}

/// A `{start, end}` record of byte offsets into the commandline.
fn span_record(start: usize, end: usize, span: Span) -> Value {
    Value::record(
        Record::from_iter([
            ("start".into(), Value::int(start as i64, span)),
            ("end".into(), Value::int(end as i64, span)),
        ]),
        span,
    )
}

/// One token of the command being completed.
struct Token {
    /// The typed text; the token under the cursor is cut there.
    text: String,
    /// `head` (the command name), `flag` (`--x`, `-x`), or `value`.
    kind: &'static str,
    /// Byte range into the line that a suggestion replacing this token replaces.
    /// Absent for alias-expanded tokens, which are not on the line.
    at: Option<(usize, usize)>,
}

impl Token {
    fn into_value(self, span: Span) -> Value {
        // Always a column: a ragged table errors on alias tokens, not `null`.
        Value::record(
            Record::from_iter([
                ("text".into(), Value::string(self.text, span)),
                ("kind".into(), Value::string(self.kind, span)),
                (
                    "span".into(),
                    self.at.map_or_else(
                        || Value::nothing(span),
                        |(start, end)| span_record(start, end, span),
                    ),
                ),
            ]),
            span,
        )
    }
}

/// Tokens of the command under the cursor, up to the cursor. Scoped to that command, so
/// `tokens.0` stays the command name past a pipe.
fn command_tokens(ctx: &Context) -> Vec<Token> {
    let cursor = ctx.buffer.len();
    let element = ctx.element.map(|element| element.span);
    let mut tokens: Vec<Token> = Vec::new();

    let flattened = ctx
        .element
        .map(|element| flatten_expression(ctx.working_set, element))
        .unwrap_or_default();

    for (token_span, shape) in &flattened {
        // On the line only if inside this command's span; alias tokens point at the definition.
        let on_line = token_span.start >= ctx.offset
            && element.is_some_and(|element| {
                token_span.start >= element.start && token_span.end <= element.end
            });

        let (text, at) = if on_line {
            let start = token_span.start - ctx.offset;
            // Never text past the cursor, though the LSP parses whole files.
            if start >= cursor {
                continue;
            }
            let end = (token_span.end - ctx.offset).min(cursor);
            (ctx.buffer[start..end].to_string(), Some((start, end)))
        } else {
            let text = ctx.working_set.get_span_contents(*token_span);
            (String::from_utf8_lossy(text).into_owned(), None)
        };

        let kind = if tokens.is_empty() {
            "head"
        } else if matches!(shape, FlatShape::Flag) || is_flag_text(&text) {
            "flag"
        } else {
            "value"
        };

        tokens.push(Token { text, kind, at });
    }

    // Append the token under the cursor; the parser may not produce one (trailing slot,
    // bare `--`), so the input record never reads `tokens | last` wrong.
    let replacing = to_reedline_span(ctx.span, ctx.offset);
    let last_is_replaced = tokens
        .last()
        .and_then(|last| last.at)
        .is_some_and(|(start, _)| start == replacing.start);

    if last_is_replaced {
        if let Some(last) = tokens.last_mut() {
            // The engine's replacement span; the LSP widens it past the typed text.
            last.at = Some((replacing.start, replacing.end));
        }
    } else {
        let text = ctx
            .buffer
            .get(replacing.start..replacing.end.min(cursor))
            .unwrap_or_default()
            .to_string();
        let kind = if tokens.is_empty() {
            "head"
        } else if is_flag_text(&text) {
            "flag"
        } else {
            "value"
        };
        tokens.push(Token {
            text,
            kind,
            at: Some((replacing.start, replacing.end)),
        });
    }

    tokens
}

/// Build the record every user completer receives; see the [module docs](self).
pub(crate) fn completer_input(ctx: &Context) -> Value {
    let span = ctx.span;
    let context_start = ctx
        .element
        .map(|element| element.span.start.saturating_sub(ctx.offset))
        .unwrap_or_else(|| ctx.buffer.len());

    Value::record(
        Record::from_iter([
            ("site".into(), ctx.site.into_value(span)),
            (
                "tokens".into(),
                Value::list(
                    command_tokens(ctx)
                        .into_iter()
                        .map(|token| token.into_value(span))
                        .collect(),
                    span,
                ),
            ),
            (
                "context_start".into(),
                Value::int(context_start as i64, span),
            ),
            ("cursor".into(), Value::int(ctx.buffer.len() as i64, span)),
        ]),
        span,
    )
}

/// The parse-free input a menu source receives; `text` is what reedline handed the menu.
/// A menu wanting `tokens`/`site` can ask via `commandline complete --input`.
pub(crate) fn menu_input(text: &str, replacing: reedline::Span, span: Span) -> Value {
    Value::record(
        Record::from_iter([
            ("text".into(), Value::string(text, span)),
            (
                "replacing".into(),
                span_record(replacing.start, replacing.end, span),
            ),
        ]),
        span,
    )
}

/// Borrow the permanent engine state when the completer lives in it (no per-keystroke
/// clone); otherwise clone it and merge the working-set delta.
fn engine_state_for_completion<'a>(
    working_set: &'a StateWorkingSet<'_>,
    is_permanent: bool,
) -> Cow<'a, EngineState> {
    if is_permanent {
        Cow::Borrowed(working_set.permanent_state)
    } else {
        let mut engine_state = working_set.permanent_state.clone();
        let _ = engine_state.merge_delta(working_set.delta.clone());
        Cow::Owned(engine_state)
    }
}

/// The block a declaration runs, seeing through aliases (a completer named by one).
fn block_of(command: &dyn Command) -> Option<BlockId> {
    command
        .block_id()
        .or_else(|| block_of(command.as_alias()?.command.as_deref()?))
}

/// A user-defined completer: a block called with the input record. Parameter,
/// command-wide, and external completers share this one implementation.
pub(crate) struct UserCompletion {
    block_id: BlockId,
    captures: Vec<(VarId, Value)>,
    narrowing: Narrowing,
}

impl UserCompletion {
    /// A completer attached to one parameter (`x: string@"nu-complete foo"`); the engine
    /// narrows its results. See [`Narrowing`].
    pub(crate) fn parameter(working_set: &StateWorkingSet<'_>, decl_id: DeclId) -> Option<Self> {
        Self::from_decl(working_set, decl_id, Narrowing::Engine)
    }

    /// A completer attached to a whole command (`@complete "nu-complete foo"`).
    pub(crate) fn command(working_set: &StateWorkingSet<'_>, decl_id: DeclId) -> Option<Self> {
        Self::from_decl(working_set, decl_id, Narrowing::Completer)
    }

    /// The configured external completer closure.
    pub(crate) fn closure(closure: &Closure) -> Self {
        Self {
            block_id: closure.block_id,
            captures: closure.captures.clone(),
            narrowing: Narrowing::Completer,
        }
    }

    /// A block-backed declaration, seeing through aliases. Builtins and plugin commands
    /// run no block and cannot serve as completers.
    fn from_decl(
        working_set: &StateWorkingSet<'_>,
        decl_id: DeclId,
        narrowing: Narrowing,
    ) -> Option<Self> {
        let block_id = (decl_id.get() < working_set.num_decls())
            .then(|| working_set.get_decl(decl_id))
            .and_then(block_of)?;

        Some(Self {
            block_id,
            captures: vec![],
            narrowing,
        })
    }

    /// Call the completer with the unified input record.
    pub(crate) fn eval(&self, ctx: &Context, input: Value) -> Result<Value, ShellError> {
        let working_set = ctx.working_set;
        let mut block = working_set.get_block(self.block_id).clone();

        // LSP completion, where a custom `def` is parsed but never compiled.
        if block.ir_block.is_none()
            && let Ok(ir_block) = compile(working_set, &block)
        {
            let mut new_block = (*block).clone();
            new_block.ir_block = Some(ir_block);
            block = Arc::new(new_block);
        }

        let mut callee_stack = ctx
            .stack
            .captures_to_stack_preserve_out_dest(self.captures.clone());

        if let Some(var_id) = block
            .signature
            .get_positional(0)
            .and_then(|positional| positional.var_id)
        {
            callee_stack.add_var(var_id, input);
        }

        let engine_state = engine_state_for_completion(
            working_set,
            self.block_id.get() < working_set.permanent_state.num_blocks(),
        );

        nu_engine::eval_block_with_early_return::<WithoutDebug>(
            engine_state.as_ref(),
            &mut callee_stack,
            &block,
            PipelineData::empty(),
        )
        .and_then(|data| data.body.into_value(ctx.span))
    }
}

impl Completer for UserCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        let value = match self.eval(ctx, completer_input(ctx)) {
            Ok(value) => value,
            Err(err) => {
                log::error!(
                    "{}",
                    ShellError::Generic(
                        GenericError::new_internal(
                            "nu::shell::completion",
                            "failed to eval completer block",
                        )
                        .with_inner([err]),
                    )
                );
                // Not an empty success: an external completer failing still lets file
                // completion answer; a parameter completer failing must not dump the
                // whole directory in place of its argument.
                return match self.narrowing {
                    Narrowing::Engine => Fetched::Cacheable(vec![]),
                    Narrowing::Completer => Fetched::Declined,
                };
            }
        };

        match CompleterOutput::read(value, ctx, self.narrowing) {
            // `null` declines, letting the next source answer.
            None => Fetched::Declined,
            Some(output) => output.into_fetched(ctx),
        }
    }
}

/// A completer's return value, after both accepted shapes are normalized.
struct CompleterOutput {
    suggestions: Vec<SemanticSuggestion>,
    options: CompletionOptions,
    sort: bool,
    filter: bool,
}

impl CompleterOutput {
    /// Read a bare list or a `{options, completions}` record; `None` when the completer
    /// declined with `null`.
    fn read(value: Value, ctx: &Context, narrowing: Narrowing) -> Option<Self> {
        let mut output = Self {
            suggestions: Vec::new(),
            options: ctx.options.clone(),
            sort: true,
            filter: narrowing.filters_by_default(),
        };
        let replacing = to_reedline_span(ctx.span, ctx.offset);

        match value {
            Value::Nothing { .. } => return None,
            Value::List { vals, .. } => {
                output.suggestions =
                    map_value_completions(vals.iter(), replacing, SpanClamp::within(ctx.buffer));
            }
            Value::Record { val, .. } => {
                if let Some(completions) = val.get("completions").and_then(|val| val.as_list().ok())
                {
                    output.suggestions = map_value_completions(
                        completions.iter(),
                        replacing,
                        SpanClamp::within(ctx.buffer),
                    );
                }
                if let Some(Value::Record { val: options, .. }) = val.get("options") {
                    output.read_options(options);
                }
            }
            other => {
                log::error!(
                    "{}",
                    ShellError::Generic(GenericError::new_internal(
                        "nu::shell::completion",
                        "completer returned invalid value",
                    )),
                );
                log::error!("completer returned type {}", other.get_type());
            }
        }

        Some(output)
    }

    /// Apply the `options` record a completer returned alongside its completions.
    fn read_options(&mut self, options: &Record) {
        if let Some(filter) = options.get("filter").and_then(|val| val.as_bool().ok()) {
            self.filter = filter;
        }

        if let Some(sort) = options.get("sort").and_then(|val| val.as_bool().ok()) {
            self.sort = sort;
            if self.sort && !self.filter {
                log::warn!("Sorting won't happen because filtering is disabled.");
            }
        }

        if let Some(case_sensitive) = options
            .get("case_sensitive")
            .and_then(|val| val.as_bool().ok())
        {
            self.options.case_sensitive = case_sensitive;
        }

        if let Some(match_description) = options
            .get("match_description")
            .and_then(|val| val.as_bool().ok())
        {
            self.options.match_description = match_description;
        }

        let positional = options.get("positional").and_then(|val| val.as_bool().ok());
        if positional.is_some() {
            log::warn!(
                "Use of the positional option is deprecated. Use the substring match algorithm instead."
            );
        }

        if let Some(algorithm) = options
            .get("completion_algorithm")
            .and_then(|option| option.coerce_string().ok())
            .and_then(|option| option.try_into().ok())
        {
            self.options.match_algorithm = algorithm;
            if let Some(false) = positional
                && self.options.match_algorithm == MatchAlgorithm::Prefix
            {
                self.options.match_algorithm = MatchAlgorithm::Substring;
            }
        }
    }

    /// Narrow the suggestions against the typed prefix, unless the completer already did.
    fn into_fetched(self, ctx: &Context) -> Fetched {
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

/// Convert a completer's values into suggestions. A record's `span` is byte offsets into
/// the commandline; `default_span` replaces when it names none. Takes the span and buffer
/// directly rather than a [`Context`] so menu sources share it.
pub(crate) fn map_value_completions<'a>(
    list: impl Iterator<Item = &'a Value>,
    default_span: reedline::Span,
    clamp: SpanClamp<'_>,
) -> Vec<SemanticSuggestion> {
    list.filter_map(move |value| {
        // Match for string values
        if let Ok(string) = value.coerce_string() {
            return Some(SemanticSuggestion {
                suggestion: Suggestion {
                    value: strip_ansi_string_unlikely(string),
                    span: default_span,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Value(value.get_type())),
            });
        }

        // Match for record values
        let Ok(record) = value.as_record() else {
            return None;
        };

        let mut suggestion = Suggestion {
            value: String::from(""),
            span: default_span,
            ..Suggestion::default()
        };
        let mut value_type = Type::String;

        for (key, value) in record.iter() {
            match key.as_str() {
                "value" => {
                    value_type = value.get_type();
                    if let Ok(string) = value.coerce_string() {
                        suggestion.value = strip_ansi_string_unlikely(string);
                    }
                }
                "display_override" => {
                    if let Ok(display) = value.coerce_string() {
                        suggestion.display_override = Some(display);
                    }
                }
                "description" => {
                    if let Ok(description) = value.coerce_string() {
                        suggestion.description = Some(description);
                    }
                }
                "style" => {
                    suggestion.style = match value {
                        Value::String { val, .. } => Some(lookup_ansi_color_style(val)),
                        Value::Record { .. } => Some(color_record_to_nustyle(value)),
                        _ => None,
                    };
                }
                "span" => {
                    if let Value::Record { val: span, .. } = value {
                        suggestion.span = read_span(span, suggestion.span, clamp);
                    }
                }
                // Extra columns beside the value, in description menus.
                "extra" => {
                    if let Value::List { vals, .. } = value {
                        suggestion.extra = Some(
                            vals.iter()
                                .filter_map(|extra| extra.coerce_string().ok())
                                .collect(),
                        );
                    }
                }
                _ => (),
            }
        }

        Some(SemanticSuggestion {
            suggestion,
            kind: Some(SuggestionKind::Value(value_type)),
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

    fn apply(&self, value: usize) -> usize {
        let value = value.min(self.limit);
        self.text.map_or(value, |text| {
            text.floor_char_boundary(value.min(text.len()))
        })
    }
}

/// Read a `{start, end}` span a completer returned. Ends clamp so it can't index out of
/// bounds or mid-character (#5127), nor replace text it was never shown.
fn read_span(
    span: &SharedCow<Record>,
    default: reedline::Span,
    clamp: SpanClamp<'_>,
) -> reedline::Span {
    let clamp = |value: usize| clamp.apply(value);

    let start = read_span_field(span, "start").map_or(default.start, clamp);
    let end = read_span_field(span, "end").map_or(default.end, clamp);

    if start > end {
        log::error!("Custom span start ({start}) is greater than end ({end})");
        return reedline::Span::new(end, end);
    }

    reedline::Span::new(start, end)
}

fn read_span_field(span: &SharedCow<Record>, field: &str) -> Option<usize> {
    let Ok(value) = span.get(field)?.as_int() else {
        log::error!("Expected span field {field} to be int");
        return None;
    };
    let Ok(value) = usize::try_from(value) else {
        log::error!("Couldn't convert span {field} to usize");
        return None;
    };

    Some(value)
}
