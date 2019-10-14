use crate::parser::hir::tokens_iterator::TokensIterator;
use crate::traits::ToDebug;

#[derive(Debug)]
pub(crate) enum DebugIteratorToken {
    Seen(String),
    Unseen(String),
    Cursor,
}

pub(crate) fn debug_tokens(iterator: &TokensIterator, source: &str) -> Vec<DebugIteratorToken> {
    let mut out = vec![];

    for (i, token) in iterator.tokens.iter().enumerate() {
        if iterator.index == i {
            out.push(DebugIteratorToken::Cursor);
        }

        if iterator.seen.contains(&i) {
            out.push(DebugIteratorToken::Seen(format!("{}", token.debug(source))));
        } else {
            out.push(DebugIteratorToken::Unseen(format!(
                "{}",
                token.debug(source)
            )));
        }
    }

    out
}
