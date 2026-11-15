use crate::completions::{
    Completer, Context, Fetched, NuMatcher, SemanticSuggestion, to_reedline_span,
};
use nu_parser::unescape_unquote_string;
use nu_protocol::{SuggestionKind, Type, engine::DEFAULT_OVERLAY_NAME};
use reedline::Suggestion;

/// Complete active, non-default overlays that can be passed to `overlay hide`.
pub struct OverlayCompletion;

impl Completer for OverlayCompletion {
    fn fetch(&mut self, ctx: &Context) -> Fetched {
        // The parser's unquoting helper expects a closing quote, even while the
        // user is still typing. An escaped double quote is part of the name.
        let mut prefix = ctx.prefix.to_vec();
        truncate_incomplete_escape(&mut prefix);
        if let Some(&quote @ (b'\'' | b'"' | b'`')) = prefix.first() {
            let escaped = quote == b'"'
                && prefix[..prefix.len() - 1]
                    .iter()
                    .rev()
                    .take_while(|&&byte| byte == b'\\')
                    .count()
                    % 2
                    == 1;
            if prefix.len() == 1 || prefix.last() != Some(&quote) || escaped {
                prefix.push(quote);
            }
        }
        let (prefix, error) = unescape_unquote_string(&prefix, ctx.span);
        if error.is_some() {
            return Fetched::answering(vec![]).without_cache();
        }
        let mut matcher = NuMatcher::literal(prefix, ctx.options, true);
        let span = to_reedline_span(ctx.span, ctx.offset);

        // Use permanent parser state: the completion parse can already hide its target.
        let engine_state = ctx.working_set.permanent_state;
        for name in &ctx.stack.active_overlays {
            if name == DEFAULT_OVERLAY_NAME
                || !engine_state
                    .active_overlay_names(&[])
                    .any(|parsed| parsed == name.as_bytes())
            {
                continue;
            }
            let (value, display_override) =
                if name.starts_with('-') || nu_utils::needs_quoting(name) {
                    (nu_utils::escape_quote_string(name), Some(name.clone()))
                } else {
                    (name.clone(), None)
                };
            matcher.add_semantic_suggestion(SemanticSuggestion {
                suggestion: Suggestion {
                    value,
                    display_override,
                    span,
                    append_whitespace: true,
                    ..Suggestion::default()
                },
                kind: Some(SuggestionKind::Value(Type::String)),
            });
        }

        // Overlay activation can change between prompts without adding declarations.
        Fetched::answering(matcher.suggestion_results()).without_cache()
    }
}

/// Match the complete part of a double-quoted prefix while an escape is still being typed.
fn truncate_incomplete_escape(prefix: &mut Vec<u8>) {
    if prefix.first() != Some(&b'"') {
        return;
    }

    let mut index = 1;
    while index < prefix.len() {
        if prefix[index] != b'\\' {
            index += 1;
            continue;
        }
        let incomplete = match &prefix[index + 1..] {
            [] | [b'u'] | [b'x'] => true,
            [b'x', digit] => digit.is_ascii_hexdigit(),
            [b'u', b'{', digits @ ..] => {
                digits.len() <= 6 && digits.iter().all(u8::is_ascii_hexdigit)
            }
            _ => false,
        };
        if incomplete {
            prefix.truncate(index);
            return;
        }
        // Skip an escaped character so a literal backslash cannot start another escape.
        index += 2;
    }
}
