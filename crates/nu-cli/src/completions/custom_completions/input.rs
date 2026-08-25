use crate::completions::{
    Context,
    completer::{CompletionContext, ResolvedCursor, is_flag_text},
    to_reedline_span,
};
use nu_parser::{FlatShape, flatten_expression};
use nu_protocol::{Signature, Span, SyntaxShape, Value, ast::Expr, record};
use std::borrow::Cow;

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
    /// The typed text; absent for a nesting token.
    text: Option<Cow<'a, str>>,
    /// `head`, `flag`, `block`, or `value`.
    kind: &'static str,
    /// Byte range the token occupies on the line; absent for alias-expanded tokens.
    at: Option<(usize, usize)>,
}

impl<'a> Token<'a> {
    /// A token spanning `start..end` of the line.
    fn on_line(
        buffer: &'a str,
        (start, end): (usize, usize),
        is_first: bool,
        is_flag_shape: bool,
    ) -> Self {
        let text = buffer.get(start..end).unwrap_or_default();
        Self {
            kind: Self::classify(text, is_first, is_flag_shape),
            text: Some(Cow::Borrowed(text)),
            at: Some((start, end)),
        }
    }

    /// A token an alias expansion produced.
    fn expanded(text: Cow<'a, str>, is_first: bool, is_flag_shape: bool) -> Self {
        Self {
            kind: Self::classify(&text, is_first, is_flag_shape),
            text: Some(text),
            at: None,
        }
    }

    /// The closure or subexpression the cursor descends into.
    fn nesting(range: (usize, usize)) -> Self {
        Self {
            text: None,
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

    /// A row of the default `tokens` table.
    fn to_value(&self, span: Span) -> Value {
        Value::record(
            record! {
                "text" => self.text.as_ref().map(|text| Value::string(text.as_ref(), span)).unwrap_or_else(|| Value::nothing(span)),
                "kind" => Value::string(self.kind, span),
                "span" => self.at.map(|(start, end)| span_record(start, end, span)).unwrap_or_else(|| Value::nothing(span)),
            },
            span,
        )
    }

    /// A row of the full form's table, including nested context.
    fn to_nested_value(&self, span: Span, nested: Option<Value>) -> Value {
        Value::record(
            record! {
                "text" => self.text.as_ref().map(|text| Value::string(text.as_ref(), span)).unwrap_or_else(|| Value::nothing(span)),
                "kind" => Value::string(self.kind, span),
                "span" => self.at.map(|(start, end)| span_record(start, end, span)).unwrap_or_else(|| Value::nothing(span)),
                "nested" => nested.unwrap_or_else(|| Value::nothing(span)),
            },
            span,
        )
    }
}

/// The tokens of one context's command.
fn context_tokens<'a>(context: &'a Context, level: &CompletionContext) -> Vec<Token<'a>> {
    let cursor = context.buffer.len();
    let element_span = level.element.map(|element| element.span);

    // Clever: Pre-allocate a reasonable capacity to avoid reallocation overhead.
    let mut tokens: Vec<Token<'a>> = Vec::with_capacity(16);

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
            let text_string = String::from_utf8_lossy(contents);
            tokens.push(Token::expanded(text_string, is_first, is_flag_shape));
        }
    }

    if let Some(descent) = level.descent {
        let start = descent.start.saturating_sub(context.offset);
        let end = descent.end.saturating_sub(context.offset).min(cursor);
        tokens.push(Token::nesting((start, end)));
        return tokens;
    }

    if locate(&tokens, cursor, Side::Trailing).is_none() {
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

#[derive(Clone, Copy)]
enum Side {
    Leading,
    Trailing,
}

/// The token `offset` falls in, and its byte offset within that token.
fn locate(tokens: &[Token], offset: usize, side: Side) -> Option<(usize, usize)> {
    let predicate = |(_, token): &(usize, &Token)| {
        token
            .at
            .is_some_and(|(start, end)| start <= offset && offset <= end)
    };

    let mut iterator = tokens.iter().enumerate();
    let (index, token) = match side {
        Side::Leading => iterator.rev().find(predicate)?,
        Side::Trailing => iterator.find(predicate)?,
    };

    let start = token
        .at
        .expect("Predicate guarantees token position is valid")
        .0;
    Some((index, offset - start))
}

fn walk_value(descents: &[usize], at: Option<(usize, usize)>, span: Span) -> Value {
    let Some((token_index, byte)) = at else {
        return Value::nothing(span);
    };

    let path = descents
        .iter()
        .copied()
        .chain(std::iter::once(token_index))
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

fn contexts_value(context: &Context, levels: &[Vec<Token<'_>>]) -> Value {
    let span = context.span;
    let mut nested_value: Option<Value> = None;

    for (level_index, tokens) in levels.iter().enumerate().rev() {
        let last_token_index = tokens.len().saturating_sub(1);
        let mut current_inner = nested_value.take();

        let rows = tokens
            .iter()
            .enumerate()
            .map(|(token_index, token)| {
                let inner_payload = (token_index == last_token_index)
                    .then(|| current_inner.take())
                    .flatten();
                token.to_nested_value(span, inner_payload)
            })
            .collect();

        let mut record = level_index
            .checked_sub(1)
            .map(|parent_index| context.contexts[parent_index].cursor.into_record(span))
            .unwrap_or_default();

        record.insert("tokens", Value::list(rows, span));
        nested_value = Some(Value::record(record, span));
    }

    nested_value.unwrap_or_else(|| Value::record(nu_protocol::Record::new(), span))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputShape {
    Token,
    Full,
}

impl InputShape {
    pub fn from_full(is_full: bool) -> Self {
        if is_full { Self::Full } else { Self::Token }
    }
}

pub(crate) fn completer_input(context: &Context, shape: InputShape) -> Value {
    match shape {
        InputShape::Token => token_input(context),
        InputShape::Full => full_input(context),
    }
}

fn token_input(context: &Context) -> Value {
    let span = context.span;
    let replacing = to_reedline_span(context.span, context.offset);
    let tokens = context
        .contexts
        .last()
        .map(|level| context_tokens(context, level))
        .unwrap_or_default();

    let token_value = locate(&tokens, context.buffer.len(), Side::Trailing)
        .map(|(index, _)| index)
        .or_else(|| tokens.len().checked_sub(1))
        .and_then(|index| tokens.get(index))
        .map(|token| token.to_value(span))
        .unwrap_or_else(|| Value::nothing(span));

    Value::record(
        record! {
            "token" => token_value,
            "place" => place_value(
                context,
                Value::int(context.buffer.len() as i64, span),
                span_record(replacing.start, replacing.end, span),
            ),
        },
        span,
    )
}

fn full_input(context: &Context) -> Value {
    let span = context.span;
    let levels: Vec<Vec<Token<'_>>> = context
        .contexts
        .iter()
        .map(|level| context_tokens(context, level))
        .collect();

    let descents: Vec<usize> = levels
        .iter()
        .rev()
        .skip(1)
        .rev()
        .map(|tokens| tokens.len().saturating_sub(1))
        .collect();

    let innermost = levels.last().map_or(&[][..], Vec::as_slice);
    let replacing = to_reedline_span(context.span, context.offset);
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
            "contexts" => contexts_value(context, &levels),
            "place" => place_value(context, walk(context.buffer.len(), Side::Trailing), target),
        },
        span,
    )
}

pub(crate) fn declared_shape(signature: &Signature) -> InputShape {
    let declares_contexts = signature
        .required_positional
        .iter()
        .chain(&signature.optional_positional)
        .any(|positional| positional.name == "contexts");

    InputShape::from_full(declares_contexts)
}
