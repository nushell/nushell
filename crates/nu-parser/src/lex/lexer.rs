use smart_default::SmartDefault;
use std::iter::Peekable;
use std::str::CharIndices;

use nu_errors::ParseError;
use nu_source::{HasSpan, Span, Spanned, SpannedItem};

use super::token_group::TokenBuilder;

use super::tokens::{
    CommandBuilder, CommentsBuilder, GroupBuilder, LiteBlock, LiteCommand, LiteComment,
    PipelineBuilder, TokenContents,
};

type Input<'t> = Peekable<CharIndices<'t>>;

#[derive(Debug, Clone)]
pub struct Token {
    pub contents: TokenContents,
    pub span: Span,
}

impl Token {
    pub fn new(contents: TokenContents, span: Span) -> Token {
        Token { contents, span }
    }
}

#[derive(Clone, Copy)]
enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
}

impl BlockKind {
    fn closing(self) -> char {
        match self {
            BlockKind::Paren => ')',
            BlockKind::SquareBracket => ']',
            BlockKind::CurlyBracket => '}',
        }
    }
}

/// Finds the extents of a basline token, returning the string with its
/// associated span, along with any parse error that was discovered along the
/// way.
///
/// Baseline tokens are unparsed content separated by spaces or a command
/// separator (like pipe or semicolon) Baseline tokens may be surrounded by
/// quotes (single, double, or backtick) or braces (square, paren, curly)
///
/// Baseline tokens may be further processed based on the needs of the syntax
/// shape that encounters them. They are still lightly lexed. For example, if a
/// baseline token begins with `{`, the entire token will continue until the
/// closing `}`, taking comments into consideration.
pub fn baseline(src: &mut Input, span_offset: usize) -> (Spanned<String>, Option<ParseError>) {
    let mut token_contents = String::new();
    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    // This variable tracks the starting character of a string literal, so that
    // we remain inside the string literal lexer mode until we encounter the
    // closing quote.
    let mut quote_start: Option<char> = None;

    let mut in_comment = false;

    // This Vec tracks paired delimiters
    let mut block_level: Vec<BlockKind> = vec![];

    // A baseline token is terminated if it's not nested inside of a paired
    // delimiter and the next character is one of: `|`, `;`, `#` or any
    // whitespace.
    fn is_termination(block_level: &[BlockKind], c: char) -> bool {
        block_level.is_empty() && (c.is_whitespace() || c == '|' || c == ';' || c == '#')
    }

    // The process of slurping up a baseline token repeats:
    //
    // - String literal, which begins with `'`, `"` or `\``, and continues until
    //   the same character is encountered again.
    // - Delimiter pair, which begins with `[`, `(`, or `{`, and continues until
    //   the matching closing delimiter is found, skipping comments and string
    //   literals.
    // - When not nested inside of a delimiter pair, when a terminating
    //   character (whitespace, `|`, `;` or `#`) is encountered, the baseline
    //   token is done.
    // - Otherwise, accumulate the character into the current baseline token.
    while let Some((_, c)) = src.peek() {
        let c = *c;

        if quote_start.is_some() {
            // If we encountered the closing quote character for the current
            // string, we're done with the current string.
            if Some(c) == quote_start {
                quote_start = None;
            }
        } else if c == '#' {
            if is_termination(&block_level, c) {
                break;
            }
            in_comment = true;
        } else if c == '\n' {
            in_comment = false;
            if is_termination(&block_level, c) {
                break;
            }
        } else if in_comment {
            if is_termination(&block_level, c) {
                break;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            // We encountered the opening quote of a string literal.
            quote_start = Some(c);
        } else if c == '[' {
            // We encountered an opening `[` delimiter.
            block_level.push(BlockKind::SquareBracket);
        } else if c == ']' {
            // We encountered a closing `]` delimiter. Pop off the opening `[`
            // delimiter.
            if let Some(BlockKind::SquareBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '{' {
            // We encountered an opening `{` delimiter.
            block_level.push(BlockKind::CurlyBracket);
        } else if c == '}' {
            // We encountered a closing `}` delimiter. Pop off the opening `{`.
            if let Some(BlockKind::CurlyBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '(' {
            // We enceountered an opening `(` delimiter.
            block_level.push(BlockKind::Paren);
        } else if c == ')' {
            // We encountered a closing `)` delimiter. Pop off the opening `(`.
            if let Some(BlockKind::Paren) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if is_termination(&block_level, c) {
            break;
        }

        // Otherwise, accumulate the character into the current token.
        token_contents.push(c);

        // Consume the character.
        let _ = src.next();
    }

    let span = Span::new(
        start_offset + span_offset,
        start_offset + span_offset + token_contents.len(),
    );

    // If there is still unclosed opening delimiters, close them and add
    // synthetic closing characters to the accumulated token.
    if let Some(block) = block_level.last() {
        let delim: char = (*block).closing();
        let cause = ParseError::unexpected_eof(delim.to_string(), span);

        while let Some(bk) = block_level.pop() {
            token_contents.push(bk.closing());
        }

        return (token_contents.spanned(span), Some(cause));
    }

    if let Some(delimiter) = quote_start {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        token_contents.push(delimiter);

        return (
            token_contents.spanned(span),
            Some(ParseError::unexpected_eof(delimiter.to_string(), span)),
        );
    }

    // If we didn't accumulate any characters, it's an unexpected error.
    if token_contents.is_empty() {
        return (
            token_contents.spanned(span),
            Some(ParseError::unexpected_eof("command".to_string(), span)),
        );
    }

    (token_contents.spanned(span), None)
}

/// We encountered a `#` character. Keep consuming characters until we encounter
/// a newline character (but don't consume it).
fn parse_comment(input: &mut Input, hash_offset: usize) -> LiteComment {
    let mut comment = String::new();
    let mut in_ws = true;
    let mut body_start = 0;

    input.next();

    while let Some((_, c)) = input.peek() {
        if *c == '\n' {
            break;
        }

        if in_ws && c.is_whitespace() {
            body_start += c.len_utf8();
        } else if in_ws && !c.is_whitespace() {
            in_ws = false;
        }

        comment.push(*c);
        input.next();
    }

    if body_start == 0 {
        let len = comment.len();

        LiteComment::new(comment.spanned(Span::new(hash_offset + 1, hash_offset + 1 + len)))
    } else {
        let ws = comment[..body_start].to_string();
        let body = comment[body_start..].to_string();

        let body_len = body.len();

        LiteComment::new_with_ws(
            ws.spanned(Span::new(hash_offset + 1, hash_offset + 1 + body_start)),
            body.spanned(Span::new(
                hash_offset + 1 + body_start,
                hash_offset + 1 + body_start + body_len,
            )),
        )
    }
}

#[derive(SmartDefault)]
struct BlockParser {
    groups: TokenBuilder<GroupBuilder>,
    group: GroupBuilder,
    pipeline: PipelineBuilder,
    command: CommandBuilder,
    prev_token: Option<Token>,
    prev_comments: CommentsBuilder,
    prev_comment_indent: usize,
}

impl BlockParser {
    fn consumed(&mut self, token: Token) {
        self.prev_token = Some(token);
    }

    fn success(mut self) -> (LiteBlock, Option<ParseError>) {
        self.close_group();

        (LiteBlock::new(self.groups.map(|g| g.into())), None)
    }

    fn fail(self, error: ParseError) -> (LiteBlock, Option<ParseError>) {
        (LiteBlock::new(self.groups.map(|g| g.into())), Some(error))
    }

    fn comment(&mut self, token: &LiteComment) {
        if self.prev_comments.is_empty() {
            self.prev_comment_indent = token.ws_len();
        }

        self.prev_comments
            .push(token.unindent(self.prev_comment_indent));
    }

    fn eoleol(&mut self) {
        self.prev_comment_indent = 0;
        self.prev_comments.take();

        self.eol();
    }

    fn eol(&mut self) {
        // If the last token on the current line is a `|`, the group
        // continues on the next line.
        if let Some(prev) = &self.prev_token {
            if let TokenContents::Pipe = prev.contents {
                return;
            }
        }

        self.close_group();
    }

    fn pipe(&mut self) -> Result<(), ()> {
        // If the current command has content, accumulate it into
        // the current pipeline and start a new command.

        match self.close_command() {
            None => Err(()),
            Some(command) => {
                self.pipeline.push(command);
                Ok(())
            }
        }
    }

    fn semicolon(&mut self) {
        self.close_pipeline();
    }

    fn baseline(&mut self, part: Spanned<String>) {
        // We encountered an unclassified character. Accumulate it into
        // the current command as a string.

        self.command.push(part);
    }

    fn close_command(&mut self) -> Option<LiteCommand> {
        let command = self.command.take()?;
        let command = LiteCommand {
            parts: command.into(),
            comments: self.prev_comments.take().map(|c| c.into()),
        };

        self.prev_comment_indent = 0;

        Some(command)
    }

    fn close_pipeline(&mut self) {
        if let Some(command) = self.close_command() {
            self.pipeline.push(command);
        }

        if let Some(pipeline) = self.pipeline.take() {
            self.group.push(pipeline);
        }
    }

    fn close_group(&mut self) {
        self.close_pipeline();

        if let Some(group) = self.group.take() {
            self.groups.push(group);
        }
    }
}

/// Try to parse a list of tokens into a block.
pub fn parse_block(tokens: Vec<Token>) -> (LiteBlock, Option<ParseError>) {
    let mut parser = BlockParser::default();

    let mut tokens = tokens.iter().peekable();

    // The parsing process repeats:
    //
    // - newline (`\n` or `\r`)
    // - pipes (`|`)
    // - semicolon
    while let Some(token) = tokens.next() {
        match &token.contents {
            TokenContents::EOL => {
                // If we encounter two newline characters in a row, use a special eoleol event,
                // which allows the parser to discard comments that shouldn't be treated as
                // documentation for the following item.
                if let Some(Token {
                    contents: TokenContents::EOL,
                    ..
                }) = tokens.peek()
                {
                    tokens.next();
                    parser.eoleol();
                } else {
                    // We encountered a newline character. If the last token on the
                    // current line is a `|`, continue the current group on the next
                    // line. Otherwise, close up the current group by rolling up the
                    // current command into the current pipeline, and then roll up
                    // the current pipeline into the group.
                    parser.eol();
                }
            }
            TokenContents::Pipe => {
                // We encountered a pipe (`|`) character, which terminates a
                // command.

                if parser.pipe().is_err() {
                    // If the current command doesn't have content, return an
                    // error that indicates that the `|` was unexpected.
                    return parser.fail(ParseError::extra_tokens(
                        "|".to_string().spanned(token.span),
                    ));
                }
                // match parser.pipe() {}
            }
            TokenContents::Semicolon => {
                // We encountered a semicolon (`;`) character, which terminates
                // a pipeline.

                parser.semicolon();
            }
            TokenContents::Baseline(part) => {
                // We encountered an unclassified character. Accumulate it into
                // the current command as a string.

                parser.baseline(part.to_string().spanned(token.span));
            }
            TokenContents::Comment(comment) => parser.comment(comment),
        }

        parser.consumed(token.clone());
    }

    parser.success()
}

/// Breaks the input string into a vector of tokens. This tokenization only tries to classify separators like
/// semicolons, pipes, etc from external bare values (values that haven't been classified further)
/// Takes in a string and and offset, which is used to offset the spans created (for when this function is used to parse inner strings)
pub fn lex(input: &str, span_offset: usize) -> (Vec<Token>, Option<ParseError>) {
    // Break the input slice into an iterator of Unicode characters.
    let mut char_indices = input.char_indices().peekable();
    let mut error = None;

    let mut output = vec![];
    let mut is_complete = true;

    // The lexing process repeats. One character of lookahead is sufficient to decide what to do next.
    //
    // - `|`: the token is either `|` token or a `||` token
    // - `;`: the token is a semicolon
    // - `\n` or `\r`: the token is an EOL (end of line) token
    // - other whitespace: ignored
    // - `#` the token starts a line comment, which contains all of the subsequent characters until the next EOL
    // -
    while let Some((idx, c)) = char_indices.peek() {
        if *c == '|' {
            // If the next character is `|`, it's either `|` or `||`.

            let idx = *idx;
            let prev_idx = idx;
            let _ = char_indices.next();

            // If the next character is `|`, we're looking at a `||`.
            if let Some((idx, c)) = char_indices.peek() {
                if *c == '|' {
                    let idx = *idx;
                    let _ = char_indices.next();
                    output.push(Token::new(
                        TokenContents::Baseline("||".into()),
                        Span::new(span_offset + prev_idx, span_offset + idx + 1),
                    ));
                    continue;
                }
            }

            // Otherwise, it's just a regular `|` token.
            output.push(Token::new(
                TokenContents::Pipe,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
            is_complete = false;
        } else if *c == ';' {
            // If the next character is a `;`, we're looking at a semicolon token.

            if !is_complete && error.is_none() {
                error = Some(ParseError::extra_tokens(
                    ";".to_string().spanned(Span::new(*idx, idx + 1)),
                ));
            }
            let idx = *idx;
            let _ = char_indices.next();
            output.push(Token::new(
                TokenContents::Semicolon,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
        } else if *c == '\n' || *c == '\r' {
            // If the next character is a newline, we're looking at an EOL (end of line) token.

            let idx = *idx;
            let _ = char_indices.next();
            output.push(Token::new(
                TokenContents::EOL,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
        } else if *c == '#' {
            // If the next character is `#`, we're at the beginning of a line
            // comment. The comment continues until the next newline.
            let idx = *idx;

            let comment = parse_comment(&mut char_indices, idx);
            let span = comment.span();

            output.push(Token::new(TokenContents::Comment(comment), span));
        } else if c.is_whitespace() {
            // If the next character is non-newline whitespace, skip it.

            let _ = char_indices.next();
        } else {
            // Otherwise, try to consume an unclassified token.

            let (result, err) = baseline(&mut char_indices, span_offset);
            if error.is_none() {
                error = err;
            }
            is_complete = true;
            let Spanned { item, span } = result;
            output.push(Token::new(TokenContents::Baseline(item), span));
        }
    }

    (output, error)
}
