use crate::completions::{
    completer::{CompletionContext, is_flag_text},
    to_reedline_span, Context,
};
use nu_parser::{flatten_expression, FlatShape};
use std::borrow::Cow;
use nu_protocol::{record, Flag, Signature, Span, Value};

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

/// One token of a context's command.
struct Token<'a> {
    /// The typed text; a token the cursor is inside is cut there. Absent for a nesting
    /// token, whose text is the command it holds as `nested`.
    text: Option<Cow<'a, str>>,
    /// `head` (the command name), `flag` (`--x`, `-x`), `block` (a nesting expression), or
    /// `value`.
    kind: &'static str,
    /// Byte range the token occupies on the line. Absent for alias-expanded tokens, which
    /// are not on it.
    at: Option<(usize, usize)>,
}

impl<'a> Token<'a> {
    /// A token spanning `start..end` of the line.
    fn on_line(buffer: &'a str, (start, end): (usize, usize), first: bool, flag_shape: bool) -> Self {
        let text = buffer.get(start..end).unwrap_or_default();
        Self {
            kind: Self::classify(text, first, flag_shape),
            text: Some(Cow::Borrowed(text)),
            at: Some((start, end)),
        }
    }

    /// A token an alias expansion produced: part of the command, but not on the line.
    fn expanded(text: Cow<'a, str>, first: bool, flag_shape: bool) -> Self {
        Self {
            kind: Self::classify(&text, first, flag_shape),
            text: Some(text),
            at: None,
        }
    }

    /// The closure or subexpression the cursor descends into. Its text is left out: it is
    /// the command carried in `nested`, which would otherwise be repeated at every level.
    fn nesting((start, end): (usize, usize)) -> Self {
        Self {
            text: None,
            kind: "block",
            at: Some((start, end)),
        }
    }

    /// `head` for the command name, else `flag` for a dash-led token and `value` otherwise.
    fn classify(text: &str, first: bool, flag_shape: bool) -> &'static str {
        if first {
            "head"
        } else if flag_shape || is_flag_text(text) {
            "flag"
        } else {
            "value"
        }
    }

    /// A row of the default `tokens` table.
    fn to_value(&self, span: Span) -> Value {
        Value::record(
            record! {
                "text" => self.text.as_ref().map_or_else(|| Value::nothing(span), |text| Value::string(text.as_ref(), span)),
                "kind" => Value::string(self.kind, span),
                "span" => self.at.map_or_else(|| Value::nothing(span), |(start, end)| span_record(start, end, span)),
            },
            span,
        )
    }

    /// A row of the full form's table, which adds the context this token holds, if any.
    fn to_nested_value(&self, span: Span, nested: Option<Value>) -> Value {
        Value::record(
            record! {
                "text" => self.text.as_ref().map_or_else(|| Value::nothing(span), |text| Value::string(text.as_ref(), span)),
                "kind" => Value::string(self.kind, span),
                "span" => self.at.map_or_else(|| Value::nothing(span), |(start, end)| span_record(start, end, span)),
                "nested" => nested.unwrap_or_else(|| Value::nothing(span)),
            },
            span,
        )
    }
}

/// The tokens of one context's command. A context the cursor only passes through stops at
/// the nesting expression it descends into, which becomes the last token; the innermost one
/// runs to the cursor and always carries a token at `replacing`.
fn context_tokens<'a>(ctx: &'a Context, level: &CompletionContext) -> Vec<Token<'a>> {
    let cursor = ctx.buffer.len();
    let element = level.element.map(|element| element.span);
    let mut tokens: Vec<Token<'a>> = Vec::new();

    let flattened = level
        .element
        .map(|element| flatten_expression(ctx.working_set, element))
        .unwrap_or_default();

    // The span of the token flattening last yielded, to drop the repeats below.
    let mut previous = None;

    for (token_span, shape) in &flattened {
        // Everything from the descent on belongs to the contexts nested inside this one.
        if level
            .descent
            .is_some_and(|descent| token_span.start >= descent.start)
        {
            break;
        }

        // Flattening yields the nodes the parser synthesized alongside the ones that were
        // typed: a missing argument is a zero-width span, and a row condition's implicit row
        // variable repeats the span of the member beside it. Neither is a token to complete.
        if token_span.start >= token_span.end || previous == Some(*token_span) {
            continue;
        }
        previous = Some(*token_span);

        // Nor is whitespace. A closure flattens to its delimiters and the padding around
        // them (`{ `, and the gap before `}`), so without this the token under a cursor
        // sitting in `each { ls ⌶` is the space itself.
        if ctx
            .working_set
            .get_span_contents(*token_span)
            .iter()
            .all(u8::is_ascii_whitespace)
        {
            continue;
        }

        // On the line only if inside this command's span; alias tokens point at the definition.
        let on_line = token_span.start >= ctx.offset
            && element.is_some_and(|element| {
                token_span.start >= element.start && token_span.end <= element.end
            });

        let first = tokens.is_empty();
        let flag_shape = matches!(shape, FlatShape::Flag);

        tokens.push(if on_line {
            let start = token_span.start - ctx.offset;
            // Never text past the cursor, though the LSP parses whole files.
            if start >= cursor {
                continue;
            }
            let end = (token_span.end - ctx.offset).min(cursor);
            Token::on_line(ctx.buffer, (start, end), first, flag_shape)
        } else {
            let text = ctx.working_set.get_span_contents(*token_span);
            Token::expanded(
                String::from_utf8_lossy(text),
                first,
                flag_shape,
            )
        });
    }

    // The nesting expression closes an enclosing context: it is the last token, and the
    // context below hangs off it.
    if let Some(descent) = level.descent {
        let start = descent.start.saturating_sub(ctx.offset);
        let end = descent.end.saturating_sub(ctx.offset).min(cursor);
        tokens.push(Token::nesting((start, end)));
        return tokens;
    }

    // The cursor can rest on no token at all: an empty slot (a trailing argument, a bare
    // `--`), or punctuation the flattener emits nothing for (the dot of `$env.⌶`). Add one
    // running to the cursor, so a walk always lands on a real token.
    if locate(&tokens, cursor, Side::Trailing).is_none() {
        let last_end = tokens
            .iter()
            .filter_map(|token| token.at)
            .map(|(_, end)| end)
            .max()
            .unwrap_or(0);

        // Never back over a token already listed: one that overlapped would put the same
        // bytes on the line twice, and out of order at that.
        let replacing = to_reedline_span(ctx.span, ctx.offset);
        let start = replacing.start.max(last_end).min(cursor);
        let first = tokens.is_empty();
        tokens.push(Token::on_line(ctx.buffer, (start, cursor), first, false));
    }

    tokens
}

/// Which side of a token boundary a byte offset belongs to when it falls exactly between
/// two tokens.
#[derive(Clone, Copy)]
enum Side {
    /// The token starting there: a range's start.
    Leading,
    /// The token ending there: a range's end, and the cursor.
    Trailing,
}

/// The token `offset` falls in, and its byte offset within that token.
fn locate(tokens: &[Token], offset: usize, side: Side) -> Option<(usize, usize)> {
    let holds = |token: &Token| {
        let (start, end) = token.at?;
        (start <= offset && offset <= end).then_some((start, end))
    };

    let mut candidates = tokens.iter().enumerate();
    let found = match side {
        Side::Leading => candidates.rev().find_map(|(index, token)| Some((index, holds(token)?))),
        Side::Trailing => candidates.find_map(|(index, token)| Some((index, holds(token)?))),
    };

    found.map(|(index, (start, _))| (index, offset - start))
}

/// A `{path, byte}` walk: `path` indexes one token per level, descending through `nested`,
/// and `byte` is the offset within the token the last index names. `descents` is the path
/// down to the context `at` is in.
///
/// Null where the offset falls on no token — `$env.⌶`, whose dot belongs to no token of its
/// own. A completer can then tell "nowhere" from a real position, which a walk pointing at
/// some token it isn't on would hide.
fn walk_value(descents: &[usize], at: Option<(usize, usize)>, span: Span) -> Value {
    let Some((token, byte)) = at else {
        return Value::nothing(span);
    };

    let path = descents
        .iter()
        .copied()
        .chain([token])
        .map(|index| Value::int(index as i64, span))
        .collect();

    Value::record(
        record! {
            "path" => Value::list(path, span),
            "byte" => Value::int(byte as i64, span),
        },
        span,
    )
}

/// Nest each level's tokens into the one above, building from the innermost out. A level's
/// nesting token is its last, and the context hanging off it is tagged with the slot it
/// fills in that level.
fn contexts_value(ctx: &Context, levels: &[Vec<Token<'_>>]) -> Value {
    let span = ctx.span;
    let mut nested: Option<Value> = None;

    for (level, tokens) in levels.iter().enumerate().rev() {
        let last = tokens.len().saturating_sub(1);
        let mut inner = nested.take();
        let rows = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| {
                token.to_nested_value(span, (index == last).then(|| inner.take()).flatten())
            })
            .collect();

        // The outermost context fills no slot: nothing encloses it.
        let mut record = match level.checked_sub(1) {
            Some(parent) => ctx.contexts[parent].cursor.into_record(span),
            None => nu_protocol::Record::new(),
        };
        record.insert("tokens", Value::list(rows, span));
        nested = Some(Value::record(record, span));
    }

    nested.unwrap_or_else(|| Value::record(nu_protocol::Record::new(), span))
}

/// Where the completion happens: the cursor, the range a suggestion replaces, and what the
/// cursor resolved to. Both shapes build it here, so the resolution sits in the same place
/// in each and only `cursor`/`target` change coordinates. See the [module docs](self).
fn place_value(ctx: &Context, cursor: Value, target: Value) -> Value {
    let span = ctx.span;
    let mut place = record! {
        "cursor" => cursor,
        "target" => target,
    };

    if let Some(level) = ctx.contexts.last() {
        place.extend(level.cursor.into_record(span));
    }

    Value::record(place, span)
}

/// Which record a completer receives. Almost every completer reads no input at all, so the
/// default is the floor and everything else is opt-in; see the [module docs](self).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputShape {
    /// `{token, place}`: the one token being completed, and where it is.
    Token,
    /// `{contexts, place}`: the nesting tree, and walks into it.
    Full,
}

impl InputShape {
    /// The shape a `--full` opt-in selects, however that opt-in was expressed: a flag a
    /// completer declares, or one typed at `commandline complete`.
    pub fn from_full(full: bool) -> Self {
        if full { Self::Full } else { Self::Token }
    }
}

/// Build the record a user completer receives; see the [module docs](self).
pub(crate) fn completer_input(ctx: &Context, shape: InputShape) -> Value {
    match shape {
        InputShape::Token => token_input(ctx),
        InputShape::Full => full_input(ctx),
    }
}

/// The default `{token, place}` record. Only the command the cursor is in is flattened, and
/// only the token under the cursor survives; the nesting around it is never walked.
fn token_input(ctx: &Context) -> Value {
    let span = ctx.span;
    let replacing = to_reedline_span(ctx.span, ctx.offset);
    let tokens = ctx
        .contexts
        .last()
        .map(|level| context_tokens(ctx, level))
        .unwrap_or_default();

    // The same token `--full` would land `place.cursor` on. The cursor can sit past every
    // token — `$env.⌶`, where the dot is no token of its own — so fall back to the last.
    let token = locate(&tokens, ctx.buffer.len(), Side::Trailing)
        .map(|(index, _)| index)
        .or_else(|| tokens.len().checked_sub(1))
        .and_then(|index| tokens.get(index))
        .map_or_else(|| Value::nothing(span), |token| token.to_value(span));

    Value::record(
        record! {
            "token" => token,
            "place" => place_value(
                ctx,
                Value::int(ctx.buffer.len() as i64, span),
                span_record(replacing.start, replacing.end, span),
            ),
        },
        span,
    )
}

/// The `--full` `{contexts, place}` record; see the [module docs](self).
fn full_input(ctx: &Context) -> Value {
    let span = ctx.span;
    let levels: Vec<Vec<Token<'_>>> = ctx
        .contexts
        .iter()
        .map(|level| context_tokens(ctx, level))
        .collect();

    // Every level but the last nests through its own last token.
    let descents: Vec<usize> = levels
        .iter()
        .rev()
        .skip(1)
        .rev()
        .map(|tokens| tokens.len().saturating_sub(1))
        .collect();

    let innermost = levels.last().map_or(&[][..], Vec::as_slice);
    let replacing = to_reedline_span(ctx.span, ctx.offset);
    let walk = |offset, side| walk_value(&descents, locate(innermost, offset, side), span);

    let target = Value::record(
        record! {
            "start" => walk(replacing.start, Side::Leading),
            "end" => walk(replacing.end, Side::Trailing),
        },
        span,
    );

    Value::record(
        record! {
            "contexts" => contexts_value(ctx, &levels),
            "place" => place_value(ctx, walk(ctx.buffer.len(), Side::Trailing), target),
        },
        span,
    )
}

/// Which record a block asked for, and the `--full` flag to bind so it can read `$full`.
/// Read off the block's own signature, so a closure — a menu source, or the external
/// completer — opts in exactly as a `def` does.
pub(crate) fn declared_shape(signature: &Signature) -> (InputShape, Option<&Flag>) {
    let full = signature
        .named
        .iter()
        .find(|flag| flag.long == "full" && flag.arg.is_none());

    (InputShape::from_full(full.is_some()), full)
}
