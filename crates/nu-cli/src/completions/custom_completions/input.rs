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

/// Flattened element tokens, plus `""` for a trailing empty slot: the old `spans`.
pub(crate) fn legacy_spans(context: &Context) -> Value {
    let Some(element) = context.contexts.last().and_then(|level| level.element) else {
        return Value::list(vec![], context.span);
    };
    let mut spans: Vec<Value> = flatten_expression(context.working_set, element)
        .iter()
        .map(|(span, _)| {
            Value::string(
                String::from_utf8_lossy(context.working_set.get_span_contents(*span)).into_owned(),
                *span,
            )
        })
        .collect();
    if context.span.is_empty() {
        spans.push(Value::string("", context.span));
    }
    Value::list(spans, context.span)
}
