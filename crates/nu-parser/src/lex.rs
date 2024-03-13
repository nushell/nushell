use nu_protocol::{ParseError, Span};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenContents {
    Item,
    Comment,
    Pipe,
    PipePipe,
    ErrGreaterPipe,
    OutErrGreaterPipe,
    Semicolon,
    OutGreaterThan,
    OutGreaterGreaterThan,
    ErrGreaterThan,
    ErrGreaterGreaterThan,
    OutErrGreaterThan,
    OutErrGreaterGreaterThan,
    Eol,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub contents: TokenContents,
    pub span: Span,
}

impl Token {
    pub fn new(contents: TokenContents, span: Span) -> Token {
        Token { contents, span }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
    AngleBracket,
}

impl BlockKind {
    fn closing(self) -> u8 {
        match self {
            BlockKind::Paren => b')',
            BlockKind::SquareBracket => b']',
            BlockKind::CurlyBracket => b'}',
            BlockKind::AngleBracket => b'>',
        }
    }
}

// A baseline token is terminated if it's not nested inside of a paired
// delimiter and the next character is one of: `|`, `;`, `#` or any
// whitespace.
fn is_item_terminator(
    block_level: &[BlockKind],
    c: u8,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
) -> bool {
    block_level.is_empty()
        && (c == b' '
            || c == b'\t'
            || c == b'\n'
            || c == b'\r'
            || c == b'|'
            || c == b';'
            || additional_whitespace.contains(&c)
            || special_tokens.contains(&c))
}

// A special token is one that is a byte that stands alone as its own token. For example
// when parsing a signature you may want to have `:` be able to separate tokens and also
// to be handled as its own token to notify you you're about to parse a type in the example
// `foo:bar`
fn is_special_item(block_level: &[BlockKind], c: u8, special_tokens: &[u8]) -> bool {
    block_level.is_empty() && special_tokens.contains(&c)
}

pub fn lex_item(
    input: &[u8],
    curr_offset: &mut usize,
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    in_signature: bool,
) -> (Token, Option<ParseError>) {
    // This variable tracks the starting character of a string literal, so that
    // we remain inside the string literal lexer mode until we encounter the
    // closing quote.
    let mut quote_start: Option<u8> = None;

    let mut in_comment = false;

    let token_start = *curr_offset;

    // This Vec tracks paired delimiters
    let mut block_level: Vec<BlockKind> = vec![];

    // The process of slurping up a baseline token repeats:
    //
    // - String literal, which begins with `'` or `"`, and continues until
    //   the same character is encountered again.
    // - Delimiter pair, which begins with `[`, `(`, or `{`, and continues until
    //   the matching closing delimiter is found, skipping comments and string
    //   literals.
    // - When not nested inside of a delimiter pair, when a terminating
    //   character (whitespace, `|`, `;` or `#`) is encountered, the baseline
    //   token is done.
    // - Otherwise, accumulate the character into the current baseline token.
    while let Some(c) = input.get(*curr_offset) {
        let c = *c;

        if let Some(start) = quote_start {
            // Check if we're in an escape sequence
            if c == b'\\' && start == b'"' {
                // Go ahead and consume the escape character if possible
                if input.get(*curr_offset + 1).is_some() {
                    // Successfully escaped the character
                    *curr_offset += 2;
                    continue;
                } else {
                    let span = Span::new(span_offset + token_start, span_offset + *curr_offset);

                    return (
                        Token {
                            contents: TokenContents::Item,
                            span,
                        },
                        Some(ParseError::UnexpectedEof(
                            (start as char).to_string(),
                            Span::new(span.end, span.end),
                        )),
                    );
                }
            }
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if c == start {
                // Also need to check to make sure we aren't escaped
                quote_start = None;
            }
        } else if c == b'#' {
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
            in_comment = true;
        } else if c == b'\n' || c == b'\r' {
            in_comment = false;
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
        } else if in_comment {
            if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
                break;
            }
        } else if is_special_item(&block_level, c, special_tokens) && token_start == *curr_offset {
            *curr_offset += 1;
            break;
        } else if c == b'\'' || c == b'"' || c == b'`' {
            // We encountered the opening quote of a string literal.
            quote_start = Some(c);
        } else if c == b'[' {
            // We encountered an opening `[` delimiter.
            block_level.push(BlockKind::SquareBracket);
        } else if c == b'<' && in_signature {
            block_level.push(BlockKind::AngleBracket);
        } else if c == b'>' && in_signature {
            if let Some(BlockKind::AngleBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == b']' {
            // We encountered a closing `]` delimiter. Pop off the opening `[`
            // delimiter.
            if let Some(BlockKind::SquareBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == b'{' {
            // We encountered an opening `{` delimiter.
            block_level.push(BlockKind::CurlyBracket);
        } else if c == b'}' {
            // We encountered a closing `}` delimiter. Pop off the opening `{`.
            if let Some(BlockKind::CurlyBracket) = block_level.last() {
                let _ = block_level.pop();
            } else {
                // We encountered a closing `}` delimiter, but the last opening
                // delimiter was not a `{`. This is an error.
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);

                *curr_offset += 1;
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(ParseError::Unbalanced(
                        "{".to_string(),
                        "}".to_string(),
                        Span::new(span.end, span.end + 1),
                    )),
                );
            }
        } else if c == b'(' {
            // We encountered an opening `(` delimiter.
            block_level.push(BlockKind::Paren);
        } else if c == b')' {
            // We encountered a closing `)` delimiter. Pop off the opening `(`.
            if let Some(BlockKind::Paren) = block_level.last() {
                let _ = block_level.pop();
            } else {
                // We encountered a closing `)` delimiter, but the last opening
                // delimiter was not a `(`. This is an error.
                let span = Span::new(span_offset + token_start, span_offset + *curr_offset);

                *curr_offset += 1;
                return (
                    Token {
                        contents: TokenContents::Item,
                        span,
                    },
                    Some(ParseError::Unbalanced(
                        "(".to_string(),
                        ")".to_string(),
                        Span::new(span.end, span.end + 1),
                    )),
                );
            }
        } else if is_item_terminator(&block_level, c, additional_whitespace, special_tokens) {
            break;
        }

        *curr_offset += 1;
    }

    let span = Span::new(span_offset + token_start, span_offset + *curr_offset);

    // If there is still unclosed opening delimiters, remember they were missing
    if let Some(block) = block_level.last() {
        let delim = block.closing();
        let cause =
            ParseError::UnexpectedEof((delim as char).to_string(), Span::new(span.end, span.end));

        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(cause),
        );
    }

    if let Some(delim) = quote_start {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(ParseError::UnexpectedEof(
                (delim as char).to_string(),
                Span::new(span.end, span.end),
            )),
        );
    }

    // If we didn't accumulate any characters, it's an unexpected error.
    if *curr_offset - token_start == 0 {
        return (
            Token {
                contents: TokenContents::Item,
                span,
            },
            Some(ParseError::UnexpectedEof("command".to_string(), span)),
        );
    }

    let mut err = None;
    let output = match &input[(span.start - span_offset)..(span.end - span_offset)] {
        b"out>" | b"o>" => Token {
            contents: TokenContents::OutGreaterThan,
            span,
        },
        b"out>>" | b"o>>" => Token {
            contents: TokenContents::OutGreaterGreaterThan,
            span,
        },
        b"err>" | b"e>" => Token {
            contents: TokenContents::ErrGreaterThan,
            span,
        },
        b"err>>" | b"e>>" => Token {
            contents: TokenContents::ErrGreaterGreaterThan,
            span,
        },
        b"out+err>" | b"err+out>" | b"o+e>" | b"e+o>" => Token {
            contents: TokenContents::OutErrGreaterThan,
            span,
        },
        b"out+err>>" | b"err+out>>" | b"o+e>>" | b"e+o>>" => Token {
            contents: TokenContents::OutErrGreaterGreaterThan,
            span,
        },
        b"&&" => {
            err = Some(ParseError::ShellAndAnd(span));
            Token {
                contents: TokenContents::Item,
                span,
            }
        }
        b"2>" => {
            err = Some(ParseError::ShellErrRedirect(span));
            Token {
                contents: TokenContents::Item,
                span,
            }
        }
        b"2>&1" => {
            err = Some(ParseError::ShellOutErrRedirect(span));
            Token {
                contents: TokenContents::Item,
                span,
            }
        }
        _ => Token {
            contents: TokenContents::Item,
            span,
        },
    };
    (output, err)
}

pub fn lex_signature(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    lex_internal(
        input,
        span_offset,
        additional_whitespace,
        special_tokens,
        skip_comment,
        true,
    )
}

pub fn lex(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
) -> (Vec<Token>, Option<ParseError>) {
    lex_internal(
        input,
        span_offset,
        additional_whitespace,
        special_tokens,
        skip_comment,
        false,
    )
}

fn lex_internal(
    input: &[u8],
    span_offset: usize,
    additional_whitespace: &[u8],
    special_tokens: &[u8],
    skip_comment: bool,
    // within signatures we want to treat `<` and `>` specially
    in_signature: bool,
) -> (Vec<Token>, Option<ParseError>) {
    let mut error = None;

    let mut curr_offset = 0;

    let mut output = vec![];
    let mut is_complete = true;

    while let Some(c) = input.get(curr_offset) {
        let c = *c;
        if c == b'|' {
            // If the next character is `|`, it's either `|` or `||`.
            let idx = curr_offset;
            let prev_idx = idx;
            curr_offset += 1;

            // If the next character is `|`, we're looking at a `||`.
            if let Some(c) = input.get(curr_offset) {
                if *c == b'|' {
                    let idx = curr_offset;
                    curr_offset += 1;
                    output.push(Token::new(
                        TokenContents::PipePipe,
                        Span::new(span_offset + prev_idx, span_offset + idx + 1),
                    ));
                    continue;
                }
            }

            // Otherwise, it's just a regular `|` token.

            // Before we push, check to see if the previous character was a newline.
            // If so, then this is a continuation of the previous line
            if let Some(prev) = output.last_mut() {
                match prev.contents {
                    TokenContents::Eol => {
                        *prev = Token::new(
                            TokenContents::Pipe,
                            Span::new(span_offset + idx, span_offset + idx + 1),
                        );
                        // And this is a continuation of the previous line if previous line is a
                        // comment line (combined with EOL + Comment)
                        //
                        // Initially, the last one token is TokenContents::Pipe, we don't need to
                        // check it, so the beginning offset is 2.
                        let mut offset = 2;
                        while output.len() > offset {
                            let index = output.len() - offset;
                            if output[index].contents == TokenContents::Comment
                                && output[index - 1].contents == TokenContents::Eol
                            {
                                output.remove(index - 1);
                                offset += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    _ => {
                        output.push(Token::new(
                            TokenContents::Pipe,
                            Span::new(span_offset + idx, span_offset + idx + 1),
                        ));
                    }
                }
            } else {
                output.push(Token::new(
                    TokenContents::Pipe,
                    Span::new(span_offset + idx, span_offset + idx + 1),
                ));
            }

            is_complete = false;
        } else if c == b';' {
            // If the next character is a `;`, we're looking at a semicolon token.

            if !is_complete && error.is_none() {
                error = Some(ParseError::ExtraTokens(Span::new(
                    curr_offset,
                    curr_offset + 1,
                )));
            }
            let idx = curr_offset;
            curr_offset += 1;
            output.push(Token::new(
                TokenContents::Semicolon,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
        } else if c == b'\r' {
            // Ignore a stand-alone carriage return
            curr_offset += 1;
        } else if c == b'\n' {
            // If the next character is a newline, we're looking at an EOL (end of line) token.
            let idx = curr_offset;
            curr_offset += 1;
            if !additional_whitespace.contains(&c) {
                output.push(Token::new(
                    TokenContents::Eol,
                    Span::new(span_offset + idx, span_offset + idx + 1),
                ));
            }
        } else if c == b'#' {
            // If the next character is `#`, we're at the beginning of a line
            // comment. The comment continues until the next newline.
            let mut start = curr_offset;

            while let Some(input) = input.get(curr_offset) {
                if *input == b'\n' {
                    if !skip_comment {
                        output.push(Token::new(
                            TokenContents::Comment,
                            Span::new(span_offset + start, span_offset + curr_offset),
                        ));
                    }
                    start = curr_offset;

                    break;
                } else {
                    curr_offset += 1;
                }
            }
            if start != curr_offset && !skip_comment {
                output.push(Token::new(
                    TokenContents::Comment,
                    Span::new(span_offset + start, span_offset + curr_offset),
                ));
            }
        } else if c == b' ' || c == b'\t' || additional_whitespace.contains(&c) {
            // If the next character is non-newline whitespace, skip it.
            curr_offset += 1;
        } else {
            let token = try_lex_special_piped_item(input, &mut curr_offset, span_offset);
            if let Some(token) = token {
                output.push(token);
                is_complete = false;
                continue;
            }

            // Otherwise, try to consume an unclassified token.
            let (token, err) = lex_item(
                input,
                &mut curr_offset,
                span_offset,
                additional_whitespace,
                special_tokens,
                in_signature,
            );
            if error.is_none() {
                error = err;
            }
            is_complete = true;
            output.push(token);
        }
    }
    (output, error)
}

/// trying to lex for the following item:
/// e>|, e+o>|, o+e>|
///
/// It returns Some(token) if we find the item, or else return None.
fn try_lex_special_piped_item(
    input: &[u8],
    curr_offset: &mut usize,
    span_offset: usize,
) -> Option<Token> {
    let c = input[*curr_offset];
    let e_pipe_len = 3;
    let eo_pipe_len = 5;
    let offset = *curr_offset;
    if c == b'e' {
        // expect `e>|`
        if (offset + e_pipe_len <= input.len()) && (&input[offset..offset + e_pipe_len] == b"e>|") {
            *curr_offset += e_pipe_len;
            return Some(Token::new(
                TokenContents::ErrGreaterPipe,
                Span::new(span_offset + offset, span_offset + offset + e_pipe_len),
            ));
        }
        if (offset + eo_pipe_len <= input.len())
            && (&input[offset..offset + eo_pipe_len] == b"e+o>|")
        {
            *curr_offset += eo_pipe_len;
            return Some(Token::new(
                TokenContents::OutErrGreaterPipe,
                Span::new(span_offset + offset, span_offset + offset + eo_pipe_len),
            ));
        }
    } else if c == b'o' {
        // it can be the following case: `o+e>|`
        if (offset + eo_pipe_len <= input.len())
            && (&input[offset..offset + eo_pipe_len] == b"o+e>|")
        {
            *curr_offset += eo_pipe_len;
            return Some(Token::new(
                TokenContents::OutErrGreaterPipe,
                Span::new(span_offset + offset, span_offset + offset + eo_pipe_len),
            ));
        }
    }
    None
}
