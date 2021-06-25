use std::sync::Arc;

use crate::{
    lex::{lexer::NewlineMode, tokens::LiteCommand},
    parse::{classify_block, util::trim_quotes},
};

use indexmap::IndexMap;
use nu_errors::ParseError;
use nu_protocol::hir::Block;
use nu_source::{HasSpan, Span, SpannedItem};

//use crate::errors::{ParseError, ParseResult};
use crate::lex::lexer::{lex, parse_block};

use crate::ParserScope;

use self::signature::parse_signature;
pub use self::signature::{lex_split_baseline_tokens_on, parse_parameter};

mod data_structs;
mod primitives;
mod signature;
mod tests;

pub(crate) fn parse_definition(call: &LiteCommand, scope: &dyn ParserScope) -> Option<ParseError> {
    // A this point, we've already handled the prototype and put it into scope;
    // So our main goal here is to parse the block now that the names and
    // prototypes of adjacent commands are also available

    match call.parts.len() {
        4 => {
            if call.parts[0].item != "def" {
                return Some(ParseError::mismatch("definition", call.parts[0].clone()));
            }

            let name = trim_quotes(&call.parts[1].item);
            let (mut signature, err) = parse_signature(&name, &call.parts[2]);

            //Add commands comments to signature usage
            signature.usage = call.comments_joined();

            if err.is_some() {
                return err;
            };

            let mut chars = call.parts[3].chars();
            match (chars.next(), chars.next_back()) {
                (Some('{'), Some('}')) => {
                    // We have a literal block
                    let string: String = chars.collect();

                    scope.enter_scope();

                    let (tokens, err) =
                        lex(&string, call.parts[3].span.start() + 1, NewlineMode::Normal);
                    if err.is_some() {
                        return err;
                    };
                    let (lite_block, err) = parse_block(tokens);
                    if err.is_some() {
                        return err;
                    };

                    let (mut block, err) = classify_block(&lite_block, scope);
                    scope.exit_scope();

                    if let Some(block) =
                        std::sync::Arc::<nu_protocol::hir::Block>::get_mut(&mut block)
                    {
                        block.params = signature;
                        block.params.name = name;
                    }

                    scope.add_definition(block);

                    err
                }
                _ => Some(ParseError::mismatch("body", call.parts[3].clone())),
            }
        }

        3 => Some(ParseError::general_error(
            "wrong shape. Expected: def name [signature] {body}",
            "expected definition body".to_string().spanned(Span::new(
                call.parts[2].span.end(),
                call.parts[2].span.end(),
            )),
        )),
        2 => Some(ParseError::general_error(
            "wrong shape. Expected: def name [signature] {body}",
            "expected definition parameters"
                .to_string()
                .spanned(Span::new(
                    call.parts[1].span.end(),
                    call.parts[1].span.end(),
                )),
        )),
        1 => Some(ParseError::general_error(
            "wrong shape. Expected: def name [signature] {body}",
            "expected definition name".to_string().spanned(Span::new(
                call.parts[0].span.end(),
                call.parts[0].span.end(),
            )),
        )),
        0 => Some(ParseError::general_error(
            "wrong shape. Expected: def name [signature] {body}",
            "expected 'def' keyword'".to_string().spanned(call.span()),
        )),

        x if x < 4 => Some(ParseError::general_error(
            "wrong shape. Expected: def name [signature] {body}",
            "expected: def name [signature] {body}"
                .to_string()
                .spanned(Span::new(
                    call.parts[x - 1].span.end(),
                    call.parts[x - 1].span.end(),
                )),
        )),
        _ => Some(ParseError::general_error(
            "extra arguments given. Expected: def name [signature] {body}.",
            "extra argument given"
                .to_string()
                .spanned(call.parts[4].span()),
        )),
    }
}

pub(crate) fn parse_definition_prototype(
    call: &LiteCommand,
    scope: &dyn ParserScope,
) -> Option<ParseError> {
    let mut err = None;

    if call.parts.len() != 4 {
        return Some(ParseError::mismatch("definition", call.parts[0].clone()));
    }

    if call.parts[0].item != "def" {
        return Some(ParseError::mismatch("definition", call.parts[0].clone()));
    }

    let name = trim_quotes(&call.parts[1].item);
    let (signature, error) = parse_signature(&name, &call.parts[2]);
    if err.is_none() {
        err = error;
    }

    scope.add_definition(Arc::new(Block::new(
        signature,
        vec![],
        IndexMap::new(),
        call.span(),
    )));

    err
}
