use log::debug;
use nu_source::Span;

use crate::lex::{Token, TokenContents};

//Currently the lexer does not split off baselines after existing text
//Example --flag(-f) is lexed as one baseline token.
//To properly parse the input, it is required that --flag and (-f) are 2 tokens.
pub(crate) fn lex_split_shortflag_from_longflag(tokens: Vec<Token>) -> Vec<Token> {
    let mut result = Vec::with_capacity(tokens.capacity());
    for token in tokens {
        let mut processed = false;
        if let TokenContents::Baseline(base) = &token.contents {
            if let Some(paren_start) = base.find('(') {
                if paren_start != 0 {
                    processed = true;
                    //If token contains '(' and '(' is not the first char,
                    //we split on '('
                    //Example: Baseline(--flag(-f)) results in: [Baseline(--flag), Baseline((-f))]
                    let paren_span_i = token.span.start() + paren_start;
                    result.push(Token::new(
                        TokenContents::Baseline(base[..paren_start].to_string()),
                        Span::new(token.span.start(), paren_span_i),
                    ));
                    result.push(Token::new(
                        TokenContents::Baseline(base[paren_start..].to_string()),
                        Span::new(paren_span_i, token.span.end()),
                    ));
                }
            }
        }

        if !processed {
            result.push(token);
        }
    }
    result
}

//Currently the lexer does not split baselines on ',' ':' '?'
//The parameter list requires this. Therefore here is a hacky method doing this.
pub(crate) fn lex_split_baseline_tokens_on(
    tokens: Vec<Token>,
    extra_baseline_terminal_tokens: &[char],
) -> Vec<Token> {
    debug!("Before lex fix up {:?}", tokens);
    let make_new_token =
        |token_new: String, token_new_end: usize, terminator_char: Option<char>| {
            let end = token_new_end;
            let start = end - token_new.len();

            let mut result = vec![];
            //Only add token if its not empty
            if !token_new.is_empty() {
                result.push(Token::new(
                    TokenContents::Baseline(token_new),
                    Span::new(start, end),
                ));
            }
            //Insert terminator_char as baseline token
            if let Some(ch) = terminator_char {
                result.push(Token::new(
                    TokenContents::Baseline(ch.to_string()),
                    Span::new(end, end + 1),
                ));
            }

            result
        };
    let mut result = Vec::with_capacity(tokens.len());
    for token in tokens {
        match token.contents {
            TokenContents::Baseline(base) => {
                let token_offset = token.span.start();
                let mut current = "".to_string();
                for (i, c) in base.chars().enumerate() {
                    if extra_baseline_terminal_tokens.contains(&c) {
                        result.extend(make_new_token(current, i + token_offset, Some(c)));
                        current = "".to_string();
                    } else {
                        current.push(c);
                    }
                }
                result.extend(make_new_token(current, base.len() + token_offset, None));
            }
            _ => result.push(token),
        }
    }
    result
}
