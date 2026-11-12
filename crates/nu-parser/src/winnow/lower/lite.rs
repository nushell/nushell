//! nu-parser's lite parse of text the lowering reads again, where nu lite-parses text again (the
//! inside of a list or a list pattern, a definition's statement).

use nu_protocol::ParseError;

use super::Lower;
use crate::winnow::syntax::LiteBlock;
use crate::winnow::syntax::lex::Token;

impl Lower<'_, '_, '_> {
    /// `tokens`, from [`Self::lex_span`], lite-parsed, with the first error nu reports there.
    pub(super) fn lite_parse(&self, tokens: &[Token]) -> (LiteBlock, Option<ParseError>) {
        let block = LiteBlock::parse_folded(self.source, tokens);
        let error = block.error().map(|error| self.lite_parse_error(error));
        (block, error)
    }
}
