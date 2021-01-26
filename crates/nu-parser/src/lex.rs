use std::str::CharIndices;
use std::{fmt, iter::Peekable};

use nu_source::{Span, Spanned, SpannedItem};

use nu_errors::ParseError;

type Input<'t> = Peekable<CharIndices<'t>>;

#[derive(Debug)]
pub struct Token {
    pub contents: TokenContents,
    pub span: Span,
}
impl Token {
    pub fn new(contents: TokenContents, span: Span) -> Token {
        Token { contents, span }
    }
}

#[derive(Clone, Debug, PartialEq, is_enum_variant)]
pub enum TokenContents {
    /// A baseline token is an atomic chunk of source code. This means that the
    /// token contains the entirety of string literals, as well as the entirety
    /// of sections delimited by paired delimiters.
    ///
    /// For example, if the token begins with `{`, the baseline token continues
    /// until the closing `}` (after taking comments and string literals into
    /// consideration).
    Baseline(String),
    Comment(String),
    Pipe,
    Semicolon,
    EOL,
}

impl fmt::Display for TokenContents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenContents::Baseline(base) => write!(f, "{}", base),
            TokenContents::Comment(comm) => write!(f, "#{}", comm),
            TokenContents::Pipe => write!(f, "|"),
            TokenContents::Semicolon => write!(f, ";"),
            TokenContents::EOL => write!(f, "\\n"),
        }
    }
}

/// A `LiteCommand` is a list of words that will get meaning when processed by
/// the parser.
#[derive(Debug, Clone)]
pub struct LiteCommand {
    pub parts: Vec<Spanned<String>>,
    ///Preceding comments. Each String in the vec is one line. The comment literal is not included.
    pub comments: Option<Vec<Spanned<String>>>,
}

impl LiteCommand {
    fn new() -> LiteCommand {
        LiteCommand {
            parts: vec![],
            comments: None,
        }
    }

    pub fn comments_joined(&self) -> String {
        match &self.comments {
            None => "".to_string(),
            Some(text) => text
                .iter()
                .map(|s| s.item.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn has_content(&self) -> bool {
        !self.is_empty()
    }

    pub fn push(&mut self, item: Spanned<String>) {
        self.parts.push(item)
    }

    pub(crate) fn span(&self) -> Span {
        let start = if let Some(x) = self.parts.first() {
            x.span.start()
        } else {
            0
        };

        let end = if let Some(x) = self.parts.last() {
            x.span.end()
        } else {
            0
        };

        Span::new(start, end)
    }
}

/// A `LitePipeline` is a series of `LiteCommand`s, separated by `|`.
#[derive(Debug, Clone)]
pub struct LitePipeline {
    pub commands: Vec<LiteCommand>,
}

impl Default for LitePipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl LitePipeline {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn has_content(&self) -> bool {
        !self.commands.is_empty()
    }

    pub fn push(&mut self, item: LiteCommand) {
        self.commands.push(item)
    }

    pub(crate) fn span(&self) -> Span {
        let start = if !self.commands.is_empty() {
            self.commands[0].span().start()
        } else {
            0
        };

        if let Some((last, _)) = self.commands[..].split_last() {
            Span::new(start, last.span().end())
        } else {
            Span::new(start, 0)
        }
    }
}

/// A `LiteGroup` is a series of `LitePipeline`s, separated by `;`.
#[derive(Debug, Clone)]
pub struct LiteGroup {
    pub pipelines: Vec<LitePipeline>,
}

impl Default for LiteGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl LiteGroup {
    pub fn new() -> Self {
        Self { pipelines: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }

    pub fn has_content(&self) -> bool {
        !self.pipelines.is_empty()
    }

    pub fn push(&mut self, item: LitePipeline) {
        self.pipelines.push(item)
    }

    #[cfg(test)]
    pub(crate) fn span(&self) -> Span {
        let start = if !self.pipelines.is_empty() {
            self.pipelines[0].span().start()
        } else {
            0
        };

        if let Some((last, _)) = self.pipelines[..].split_last() {
            Span::new(start, last.span().end())
        } else {
            Span::new(start, 0)
        }
    }
}

/// A `LiteBlock` is a series of `LiteGroup`s, separated by newlines.
#[derive(Debug, Clone)]
pub struct LiteBlock {
    pub block: Vec<LiteGroup>,
}

impl LiteBlock {
    pub fn new(block: Vec<LiteGroup>) -> Self {
        Self { block }
    }

    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }

    pub fn push(&mut self, item: LiteGroup) {
        self.block.push(item)
    }

    #[cfg(test)]
    pub(crate) fn span(&self) -> Span {
        let start = if !self.block.is_empty() {
            self.block[0].span().start()
        } else {
            0
        };

        if let Some((last, _)) = self.block[..].split_last() {
            Span::new(start, last.span().end())
        } else {
            Span::new(start, 0)
        }
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

/// Try to parse a list of tokens into a block.
pub fn block(tokens: Vec<Token>) -> (LiteBlock, Option<ParseError>) {
    // Accumulate chunks of tokens into groups.
    let mut groups = vec![];

    // The current group
    let mut group = LiteGroup::new();

    // The current pipeline
    let mut pipeline = LitePipeline::new();

    // The current command
    let mut command = LiteCommand::new();

    let mut prev_comments = None;
    let mut prev_comment_indent = 0;

    let mut prev_token: Option<Token> = None;

    // The parsing process repeats:
    //
    // - newline (`\n` or `\r`)
    // - pipes (`|`)
    // - semicolon
    fn finish_command(
        prev_comments: &mut Option<Vec<Spanned<String>>>,
        command: &mut LiteCommand,
        pipeline: &mut LitePipeline,
    ) {
        if let Some(prev_comments_) = prev_comments {
            //Add previous comments to this command
            command.comments = Some(prev_comments_.clone());
            //Reset
            *prev_comments = None;
        }
        pipeline.push(command.clone());
        *command = LiteCommand::new();
    }

    for token in tokens {
        match &token.contents {
            TokenContents::EOL => {
                // We encountered a newline character. If the last token on the
                // current line is a `|`, continue the current group on the next
                // line. Otherwise, close up the current group by rolling up the
                // current command into the current pipeline, and then roll up
                // the current pipeline into the group.

                // If the last token on the current line is a `|`, the group
                // continues on the next line.
                if let Some(prev) = &prev_token {
                    if let TokenContents::Pipe = prev.contents {
                        continue;
                    }
                    if let TokenContents::EOL = prev.contents {
                        //If we have an empty line we discard previous comments as they are not
                        //part of a command
                        //Example nu Code:
                        //#I am a comment getting discarded
                        //
                        //def e [] {echo hi}
                        prev_comments = None
                    }
                }

                // If we have an open command, push it into the current
                // pipeline.
                if command.has_content() {
                    finish_command(&mut prev_comments, &mut command, &mut pipeline);
                }

                // If we have an open pipeline, push it into the current group.
                if pipeline.has_content() {
                    group.push(pipeline);
                    pipeline = LitePipeline::new();
                }

                // If we have an open group, accumulate it into `groups`.
                if group.has_content() {
                    groups.push(group);
                    group = LiteGroup::new();
                }
            }
            TokenContents::Pipe => {
                // We encountered a pipe (`|`) character, which terminates a
                // command.

                // If the current command has content, accumulate it into
                // the current pipeline and start a new command.
                if command.has_content() {
                    finish_command(&mut prev_comments, &mut command, &mut pipeline);
                } else {
                    // If the current command doesn't have content, return an
                    // error that indicates that the `|` was unexpected.
                    return (
                        LiteBlock::new(groups),
                        Some(ParseError::extra_tokens(
                            "|".to_string().spanned(token.span),
                        )),
                    );
                }
            }
            TokenContents::Semicolon => {
                // We encountered a semicolon (`;`) character, which terminates
                // a pipeline.

                // If the current command has content, accumulate it into the
                // current pipeline and start a new command.
                if command.has_content() {
                    finish_command(&mut prev_comments, &mut command, &mut pipeline);
                }

                // If the current pipeline has content, accumulate it into the
                // current group and start a new pipeline.
                if pipeline.has_content() {
                    group.push(pipeline);
                    pipeline = LitePipeline::new();
                }
            }
            TokenContents::Baseline(bare) => {
                // We encountered an unclassified character. Accumulate it into
                // the current command as a string.

                command.push(bare.to_string().spanned(token.span));
            }
            TokenContents::Comment(comment) => {
                if prev_comments.is_none() {
                    //Calculate amount of space indent
                    if let Some((i, _)) = comment.chars().enumerate().find(|(_, ch)| *ch != ' ') {
                        prev_comment_indent = i;
                    }
                }
                let comment: String = comment
                    .chars()
                    .enumerate()
                    .skip_while(|(i, ch)| *i < prev_comment_indent && *ch == ' ')
                    .map(|(_, ch)| ch)
                    .collect();

                //Because we skipped some spaces at start, the span needs to be adjusted
                let comment_span = Span::new(token.span.end() - comment.len(), token.span.end());

                prev_comments
                    .get_or_insert(vec![])
                    .push(comment.spanned(comment_span));
            }
        }
        prev_token = Some(token);
    }

    // If the current command has content, accumulate it into the current pipeline.
    if command.has_content() {
        finish_command(&mut prev_comments, &mut command, &mut pipeline)
    }

    // If the current pipeline has content, accumulate it into the current group.
    if pipeline.has_content() {
        group.push(pipeline);
    }

    // If the current group has content, accumulate it into the list of groups.
    if group.has_content() {
        groups.push(group);
    }

    // Return a new LiteBlock with the accumulated list of groups.
    (LiteBlock::new(groups), None)
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
            let comment_start = *idx + 1;
            let mut comment = String::new();
            //Don't copy '#' into comment string
            char_indices.next();
            while let Some((_, c)) = char_indices.peek() {
                if *c == '\n' {
                    break;
                }
                comment.push(*c);
                //Advance char_indices
                let _ = char_indices.next();
            }
            let token = Token::new(
                TokenContents::Comment(comment.clone()),
                Span::new(
                    span_offset + comment_start,
                    span_offset + comment_start + comment.len(),
                ),
            );
            output.push(token);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn span(left: usize, right: usize) -> Span {
        Span::new(left, right)
    }

    mod bare {
        use super::*;

        #[test]
        fn simple_1() {
            let input = "foo bar baz";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 3));
        }

        #[test]
        fn simple_2() {
            let input = "'foo bar' baz";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 9));
        }

        #[test]
        fn simple_3() {
            let input = "'foo\" bar' baz";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 10));
        }

        #[test]
        fn simple_4() {
            let input = "[foo bar] baz";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 9));
        }

        #[test]
        fn simple_5() {
            let input = "'foo 'bar baz";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 9));
        }

        #[test]
        fn simple_6() {
            let input = "''foo baz";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 5));
        }

        #[test]
        fn simple_7() {
            let input = "'' foo";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 2));
        }

        #[test]
        fn simple_8() {
            let input = " '' foo";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(1, 3));
        }

        #[test]
        fn simple_9() {
            let input = " 'foo' foo";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(1, 6));
        }

        #[test]
        fn simple_10() {
            let input = "[foo, bar]";

            let (result, err) = lex(input, 0);

            assert!(err.is_none());
            assert_eq!(result[0].span, span(0, 10));
        }

        #[test]
        fn lex_comment() {
            let input = r#"
#A comment
def e [] {echo hi}
                "#;

            let (result, err) = lex(input, 0);
            assert!(err.is_none());
            //result[0] == EOL
            assert_eq!(result[1].span, span(2, 11));
            assert_eq!(
                result[1].contents,
                TokenContents::Comment("A comment".to_string())
            );
        }

        #[test]
        fn ignore_future() {
            let input = "foo 'bar";

            let (result, _) = lex(input, 0);

            assert_eq!(result[0].span, span(0, 3));
        }

        #[test]
        fn invalid_1() {
            let input = "'foo bar";

            let (_, err) = lex(input, 0);

            assert!(err.is_some());
        }

        #[test]
        fn invalid_2() {
            let input = "'bar";

            let (_, err) = lex(input, 0);

            assert!(err.is_some());
        }

        #[test]
        fn invalid_4() {
            let input = " 'bar";

            let (_, err) = lex(input, 0);

            assert!(err.is_some());
        }
    }

    mod lite_parse {
        use super::*;

        #[test]
        fn pipeline() {
            let (result, err) = lex("cmd1 | cmd2 ; deploy", 0);
            assert!(err.is_none());
            let (result, err) = block(result);
            assert!(err.is_none());
            assert_eq!(result.span(), span(0, 20));
            assert_eq!(result.block[0].pipelines[0].span(), span(0, 11));
            assert_eq!(result.block[0].pipelines[1].span(), span(14, 20));
        }

        #[test]
        fn simple_1() {
            let (result, err) = lex("foo", 0);
            assert!(err.is_none());
            let (result, err) = block(result);
            assert!(err.is_none());
            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].pipelines.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 1);
            assert_eq!(
                result.block[0].pipelines[0].commands[0].parts[0].span,
                span(0, 3)
            );
        }

        #[test]
        fn simple_offset() {
            let (result, err) = lex("foo", 10);
            assert!(err.is_none());
            let (result, err) = block(result);
            assert!(err.is_none());
            assert_eq!(result.block[0].pipelines.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 1);
            assert_eq!(
                result.block[0].pipelines[0].commands[0].parts[0].span,
                span(10, 13)
            );
        }

        #[test]
        fn incomplete_result() {
            let (result, err) = lex("my_command \"foo' --test", 10);
            assert!(matches!(err.unwrap().reason(), nu_errors::ParseErrorReason::Eof { .. }));
            let (result, _) = block(result);

            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].pipelines.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);

            assert_eq!(
                result.block[0].pipelines[0].commands[0].parts[0].item,
                "my_command"
            );
            assert_eq!(
                result.block[0].pipelines[0].commands[0].parts[1].item,
                "\"foo' --test\""
            );
        }
        #[test]
        fn command_with_comment() {
            let code = r#"
# My echo
# * It's much better :)
def my_echo [arg] { echo $arg }
            "#;
            let (result, err) = lex(code, 0);
            assert!(err.is_none());
            let (result, err) = block(result);
            assert!(err.is_none());

            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].pipelines.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 4);
            assert_eq!(
                result.block[0].pipelines[0].commands[0].comments,
                Some(vec![
                    //Leading space is trimmed
                    "My echo".to_string().spanned(Span::new(3, 10)),
                    "* It's much better :)"
                        .to_string()
                        .spanned(Span::new(13, 34))
                ])
            );
        }
        #[test]
        fn discarded_comment() {
            let code = r#"
# This comment gets discarded, because of the following empty line

echo 42
            "#;
            let (result, err) = lex(code, 0);
            assert!(err.is_none());
            // assert_eq!(format!("{:?}", result), "");
            let (result, err) = block(result);
            assert!(err.is_none());
            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].pipelines.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
            assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);
            assert_eq!(result.block[0].pipelines[0].commands[0].comments, None);
        }
    }

    #[test]
    fn no_discarded_white_space_start_of_comment() {
        let code = r#"
#No white_space at firt line ==> No white_space discarded
#   Starting space is not discarded
echo 42
            "#;
        let (result, err) = lex(code, 0);
        assert!(err.is_none());
        // assert_eq!(format!("{:?}", result), "");
        let (result, err) = block(result);
        assert!(err.is_none());
        assert_eq!(result.block.len(), 1);
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);
        assert_eq!(
            result.block[0].pipelines[0].commands[0].comments,
            Some(vec![
                "No white_space at firt line ==> No white_space discarded"
                    .to_string()
                    .spanned(Span::new(2, 58)),
                "   Starting space is not discarded"
                    .to_string()
                    .spanned(Span::new(60, 94)),
            ])
        );
    }

    #[test]
    fn multiple_discarded_white_space_start_of_comment() {
        let code = r#"
#  Discard 2 spaces
# Discard 1 space
#  Discard 2 spaces
echo 42
            "#;
        let (result, err) = lex(code, 0);
        assert!(err.is_none());
        // assert_eq!(format!("{:?}", result), "");
        let (result, err) = block(result);
        assert!(err.is_none());
        assert_eq!(result.block.len(), 1);
        assert_eq!(result.block[0].pipelines.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands.len(), 1);
        assert_eq!(result.block[0].pipelines[0].commands[0].parts.len(), 2);
        assert_eq!(
            result.block[0].pipelines[0].commands[0].comments,
            Some(vec![
                "Discard 2 spaces".to_string().spanned(Span::new(4, 20)),
                "Discard 1 space".to_string().spanned(Span::new(23, 38)),
                "Discard 2 spaces".to_string().spanned(Span::new(42, 58)),
            ])
        );
    }
}
