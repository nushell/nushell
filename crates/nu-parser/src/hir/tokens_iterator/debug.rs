#![allow(unused)]

pub(crate) mod color_trace;
pub(crate) mod expand_trace;

pub(crate) use self::color_trace::*;
pub(crate) use self::expand_trace::*;

use crate::hir::tokens_iterator::TokensIteratorState;
use nu_source::{PrettyDebug, PrettyDebugWithSource, Text};

#[derive(Debug)]
pub(crate) enum DebugIteratorToken {
    Seen(String),
    Unseen(String),
    Cursor,
}

pub(crate) fn debug_tokens(state: &TokensIteratorState, source: &str) -> Vec<DebugIteratorToken> {
    let mut out = vec![];

    for (i, token) in state.tokens.iter().enumerate() {
        if state.index == i {
            out.push(DebugIteratorToken::Cursor);
        }

        let msg = token.debug(source).to_string();
        if state.seen.contains(&i) {
            out.push(DebugIteratorToken::Seen(msg));
        } else {
            out.push(DebugIteratorToken::Unseen(msg));
        }
    }

    out
}
