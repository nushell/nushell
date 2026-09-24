use crate::completions::{
    Context,
    completer::{CompletionContext, ResolvedCursor, is_flag_text},
    to_reedline_span,
};
use nu_parser::{FlatShape, flatten_expression};
use nu_protocol::{Record, Signature, Span, SyntaxShape, Value, ast::Expr, record};

/// A `{start, end}` record of byte offsets into the commandline.
fn span_record(start: usize, end: usize, span: Span) -> Value {
    Value::record(
        record! {
            "start" => Value::int(start as i64, span),
            "end" => Value::int(end as i64, span),
        },
        span,
    )
}

/// Token kind and span.
struct Token {
    /// `head`, `flag`, `block`, or `value`.
    kind: &'static str,
    /// Byte range the token occupies on the line; absent for alias-expanded tokens.
    at: Option<(usize, usize)>,
}

impl Token {
    /// A token spanning `start..end` of the line.
    fn on_line(
        buffer: &str,
        (start, end): (usize, usize),
        is_first: bool,
        is_flag_shape: bool,
    ) -> Self {
        Self {
            kind: Self::classify(
                buffer.get(start..end).unwrap_or_default(),
                is_first,
                is_flag_shape,
            ),
            at: Some((start, end)),
        }
    }

    /// A token an alias expansion produced.
    fn expanded(text: &str, is_first: bool, is_flag_shape: bool) -> Self {
        Self {
            kind: Self::classify(text, is_first, is_flag_shape),
            at: None,
        }
    }

    /// The closure or subexpression the cursor descends into.
    fn nesting(range: (usize, usize)) -> Self {
        Self {
            kind: "block",
            at: Some(range),
        }
    }

    /// Classifies the token strictly based on position and shape.
    fn classify(text: &str, is_first: bool, is_flag_shape: bool) -> &'static str {
        if is_first {
            "head"
        } else if is_flag_shape || is_flag_text(text) {
            "flag"
        } else {
            "value"
        }
    }
}

/// The tokens of one context's command.
fn context_tokens(context: &Context, level: &CompletionContext) -> Vec<Token> {
    let cursor = context.buffer.len();
    let element_span = level.element.map(|element| element.span);

    // Clever: Pre-allocate a reasonable capacity to avoid reallocation overhead.
    let mut tokens: Vec<Token> = Vec::with_capacity(16);

    let flattened = level
        .element
        .map(|element| flatten_expression(context.working_set, element))
        .unwrap_or_default();

    let mut previous_span = None;

    for (token_span, shape) in &flattened {
        // Break exactly when we cross into nested territory.
        if level
            .descent
            .is_some_and(|descent| token_span.start >= descent.start)
        {
            break;
        }

        // Bypass zero-width or duplicated implicit variable spans cleanly.
        if token_span.start >= token_span.end || previous_span == Some(*token_span) {
            continue;
        }
        previous_span = Some(*token_span);

        let contents = context.working_set.get_span_contents(*token_span);
        if contents.iter().all(u8::is_ascii_whitespace) {
            continue;
        }

        let is_on_line = token_span.start >= context.offset
            && element_span.is_some_and(|element| {
                token_span.start >= element.start && token_span.end <= element.end
            });

        let is_first = tokens.is_empty();
        let is_flag_shape = matches!(shape, FlatShape::Flag);

        if is_on_line {
            let start = token_span.start.saturating_sub(context.offset);
            if start >= cursor {
                continue;
            }
            let end = (token_span.end.saturating_sub(context.offset)).min(cursor);
            tokens.push(Token::on_line(
                context.buffer,
                (start, end),
                is_first,
                is_flag_shape,
            ));
        } else {
            let text = String::from_utf8_lossy(contents);
            tokens.push(Token::expanded(&text, is_first, is_flag_shape));
        }
    }

    if let Some(descent) = level.descent {
        let start = descent.start.saturating_sub(context.offset);
        let end = descent.end.saturating_sub(context.offset).min(cursor);
        tokens.push(Token::nesting((start, end)));
        return tokens;
    }

    if locate(&tokens, cursor).is_none() {
        // Clever: We can safely grab the last known end point because tokens are appended sequentially.
        let last_end = tokens
            .iter()
            .filter_map(|token| token.at)
            .next_back()
            .map(|(_, end)| end)
            .unwrap_or(0);
        let replacing = to_reedline_span(context.span, context.offset);
        let start = replacing.start.max(last_end).min(cursor);

        tokens.push(Token::on_line(
            context.buffer,
            (start, cursor),
            tokens.is_empty(),
            false,
        ));
    }

    tokens
}

/// The token `offset` falls in, and its byte offset within that token.
fn locate(tokens: &[Token], offset: usize) -> Option<(usize, usize)> {
    let (index, token) = tokens.iter().enumerate().find(|(_, token)| {
        token
            .at
            .is_some_and(|(start, end)| start <= offset && offset <= end)
    })?;

    let start = token
        .at
        .expect("Predicate guarantees token position is valid")
        .0;
    Some((index, offset - start))
}

/// Return the declared shape for the argument under the cursor.
fn expected_shape(context: &Context, level: &CompletionContext) -> Option<SyntaxShape> {
    let Expr::Call(call) = &level.element?.expr else {
        return None;
    };
    let signature = context.working_set.get_decl(call.decl_id).signature();

    match level.cursor {
        ResolvedCursor::Positional { index } => signature
            .get_positional(index)
            .map(|positional| positional.shape.clone()),
        // `flag` may be either the long name or a short-only flag.
        ResolvedCursor::FlagValue { flag } => signature
            .get_long_flag(flag)
            .or_else(|| {
                flag.chars()
                    .next()
                    .and_then(|c| signature.get_short_flag(c))
            })
            .and_then(|flag| flag.arg),
        _ => None,
    }
}

fn place_value(context: &Context, cursor: Value, target: Value) -> Value {
    let span = context.span;
    let mut place = record! {
        "cursor" => cursor,
        "target" => target,
    };

    if let Some(level) = context.contexts.last() {
        place.extend(level.cursor.into_record(span));
        if let Some(shape) = expected_shape(context, level) {
            place.insert("shape", Value::string(shape.to_string(), span));
        }
    }

    // The command the cursor is in, so completers need not re-parse `buffer`.
    // One syntactic token per argument (no flatten), so `token`'s filtered walk
    // is never paid for when only `place` is asked for.
    place.insert("command", command_tokens(context));

    Value::record(place, span)
}

/// Which of `token`, `place`, and `buffer` a completer asked for. The input record is
/// overloaded on this: each field is computed and handed over only when it is declared, so a
/// completer is never given -- and never pays for -- a field it did not ask for.
#[derive(Debug, Clone, Copy)]
pub struct DeclaredInputs {
    pub token: bool,
    pub place: bool,
    pub buffer: bool,
}

impl DeclaredInputs {
    /// Every field; what `commandline complete --input` reports for inspection.
    pub fn all() -> Self {
        Self {
            token: true,
            place: true,
            buffer: true,
        }
    }

    /// The recognized input fields a completer's signature declares as positionals.
    pub(crate) fn from_signature(signature: &Signature) -> Self {
        let declares = |name: &str| {
            signature
                .required_positional
                .iter()
                .chain(&signature.optional_positional)
                .any(|positional| positional.name == name)
        };
        Self {
            token: declares("token"),
            place: declares("place"),
            buffer: declares("buffer"),
        }
    }
}

/// Token text/span/kind for replacement range.
fn token_value(context: &Context) -> Value {
    let span = context.span;
    let replacing = to_reedline_span(span, context.offset);
    let tokens = context
        .contexts
        .last()
        .map(|level| context_tokens(context, level))
        .unwrap_or_default();

    let kind = locate(&tokens, context.buffer.len())
        .map(|(index, _)| index)
        .or_else(|| tokens.len().checked_sub(1))
        .and_then(|index| tokens.get(index))
        .map_or("value", |token| token.kind);

    let (start, end) = (replacing.start, replacing.end.min(context.buffer.len()));
    let text = context.buffer.get(start..end).unwrap_or_default();

    Value::record(
        record! {
            "text" => Value::string(text, span),
            "kind" => Value::string(kind, span),
            "span" => span_record(start, end, span),
        },
        span,
    )
}

/// Place: cursor, target, and resolved site.
fn place_of(context: &Context) -> Value {
    let span = context.span;
    let replacing = to_reedline_span(span, context.offset);
    place_value(
        context,
        Value::int(context.buffer.len() as i64, span),
        span_record(replacing.start, replacing.end, span),
    )
}

/// Record for completer with only declared fields.
pub(crate) fn completer_input(context: &Context, wanted: DeclaredInputs) -> Value {
    let span = context.span;
    let mut record = Record::new();

    if wanted.token {
        record.insert("token", token_value(context));
    }
    if wanted.place {
        record.insert("place", place_of(context));
    }
    if wanted.buffer {
        record.insert("buffer", Value::string(context.buffer, span));
    }

    Value::record(record, span)
}

/// our legacy compat shim!
pub(crate) fn legacy_context(context: &Context) -> Value {
    let cursor = context.offset + context.buffer.len();
    let start = context
        .contexts
        .last()
        .and_then(|level| level.element)
        .map(|element| element.span.start)
        .unwrap_or(cursor)
        .clamp(context.offset, cursor);
    let text = String::from_utf8_lossy(
        context
            .working_set
            .get_span_contents(Span::new(start, cursor)),
    )
    .into_owned();
    Value::string(text, context.span)
}

/// Buffer-relative cursor, i.e. the old second positional (`pos`/`position`).
pub(crate) fn legacy_pos(context: &Context) -> Value {
    Value::int(context.buffer.len() as i64, context.span)
}

/// Tokens of the command the cursor is in, plus `""` for a trailing empty slot.
///
/// Backs `place.command`; also the legacy `spans` value. One shell word per
/// argument (`foo {|x| $x } fil` is `[foo, "{|x| $x }", fil]`, never empty.
pub(crate) fn command_tokens(context: &Context) -> Value {
    PlaceCommand::of(context).into_value(context.span)
}

/// argv of the command the cursor is in. Always non-empty, with a trailing `""`
/// iff the cursor sits on an empty slot, so `$place.command.0` never index-errors.
struct PlaceCommand(Vec<String>);

impl PlaceCommand {
    fn of(context: &Context) -> Self {
        let line = Line::of(context);
        let mut words: Vec<String> = match context.contexts.last().and_then(|level| level.element) {
            Some(element) => match &element.expr {
                Expr::Call(call) => line
                    .word(call.head)
                    .into_iter()
                    .chain(
                        call.arguments
                            .iter()
                            .flat_map(|arg| arg_pair(&line, arg).into_iter().flatten()),
                    )
                    .collect(),
                Expr::ExternalCall(head, args) => line
                    .word(head.span)
                    .into_iter()
                    .chain(args.iter().filter_map(|arg| match arg {
                        nu_protocol::ast::ExternalArgument::Regular(e) => line.word(e.span),
                        nu_protocol::ast::ExternalArgument::Spread(e) => line.spread(e.span),
                    }))
                    .collect(),
                // A bare word, variable, or cell path is its own command.
                _ => line.word(element.span).into_iter().collect(),
            },
            None => Vec::new(),
        };
        if context.span.is_empty() && !matches!(words.last(), Some(w) if w.is_empty()) {
            words.push(String::new());
        }
        if words.is_empty() {
            words.push(String::new());
        }
        Self(words)
    }

    fn into_value(self, span: Span) -> Value {
        Value::list(
            self.0
                .into_iter()
                .map(|word| Value::string(word, span))
                .collect(),
            span,
        )
    }
}

/// One argument as zero to two shell words: flags stay whole (`--flag=value`) or
/// split (`--flag value`); everything else is one word or skipped when empty.
fn arg_pair(line: &Line, arg: &nu_protocol::ast::Argument) -> [Option<String>; 2] {
    use nu_protocol::ast::Argument;
    match arg {
        Argument::Positional(e) | Argument::Unknown(e) => [line.word(e.span), None],
        Argument::Spread(e) => [line.spread(e.span), None],
        Argument::Named((long, short, value)) => {
            // Long and short cover the same source text; reading both would double it.
            let long_word = line.word(long.span);
            let (flag_span, flag) = match (long_word, short) {
                (Some(flag), _) => (long.span, flag),
                (None, Some(short)) => match line.word(short.span) {
                    Some(flag) => (short.span, flag),
                    None => return [None, value.as_ref().and_then(|v| line.word(v.span))],
                },
                (None, None) => return [None, value.as_ref().and_then(|v| line.word(v.span))],
            };
            let Some(value) = value else {
                return [Some(flag), None];
            };
            // Joined (`--flag=value`, gap `=`) is one word; spaced gaps are two,
            // so `--flag foo=bar` never collapses even though the value holds `=`.
            if line.spaced(flag_span.end, value.span.start) {
                [Some(flag), line.word(value.span)]
            } else {
                [line.word(arg.span()), None]
            }
        }
    }
}

/// The line being completed: clips working-set spans to it. `None` means empty
/// or off-line (whitespace, synthetic, alias-expanded), so empties never leak.
struct Line<'a> {
    working_set: &'a nu_protocol::engine::StateWorkingSet<'a>,
    offset: usize,
    end: usize,
}

impl<'a> Line<'a> {
    fn of(context: &'a Context) -> Self {
        Self {
            working_set: context.working_set,
            offset: context.offset,
            end: context.offset + context.buffer.len(),
        }
    }

    fn word(&self, span: Span) -> Option<String> {
        if span.start >= span.end {
            return None;
        }
        let end = span.end.min(self.end);
        let start = span.start.max(self.offset).min(end);
        if start >= end {
            return None;
        }
        let text =
            String::from_utf8_lossy(self.working_set.get_span_contents(Span::new(start, end)))
                .into_owned();
        (!text.is_empty()).then_some(text)
    }

    /// Whether the gap between a flag and its value holds whitespace: the
    /// shell-word boundary (`=`-joined gaps hold only `=`).
    fn spaced(&self, flag_end: usize, value_start: usize) -> bool {
        if flag_end >= value_start {
            return false;
        }
        let (start, end) = (flag_end.max(self.offset), value_start.min(self.end));
        start < end
            && self
                .working_set
                .get_span_contents(Span::new(start, end))
                .iter()
                .any(u8::is_ascii_whitespace)
    }

    /// A spread as one word, keeping the `...` prefix that lives outside the
    /// inner span (`...$args`, not `$args`).
    fn spread(&self, inner: Span) -> Option<String> {
        let mut word = self.word(inner)?;
        if inner.start >= self.offset + 3
            && self
                .working_set
                .get_span_contents(Span::new(inner.start - 3, inner.start))
                == b"..."
        {
            word.insert_str(0, "...");
        }
        Some(word)
    }
}
