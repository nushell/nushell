use std::iter::Peekable;
use std::str::CharIndices;

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

#[derive(Debug)]
pub enum TokenContents {
    Bare(String),
    Pipe,
    Semicolon,
    EOL,
}

#[derive(Debug, Clone)]
pub struct LiteCommand {
    pub parts: Vec<Spanned<String>>,
}

impl LiteCommand {
    fn new() -> LiteCommand {
        LiteCommand { parts: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
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

#[derive(Debug, Clone)]
pub struct LitePipeline {
    pub commands: Vec<LiteCommand>,
}

impl LitePipeline {
    pub fn new() -> Self {
        Self { commands: vec![] }
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
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

#[derive(Debug, Clone)]
pub struct LiteGroup {
    pub pipelines: Vec<LitePipeline>,
}

impl LiteGroup {
    pub fn new() -> Self {
        Self { pipelines: vec![] }
    }
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
    pub fn push(&mut self, item: LitePipeline) {
        self.pipelines.push(item)
    }
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

#[derive(Debug, Clone)]
pub struct LiteBlock {
    pub block: Vec<LiteGroup>,
}

impl LiteBlock {
    pub fn new() -> Self {
        Self { block: vec![] }
    }
    pub fn is_empty(&self) -> bool {
        self.block.is_empty()
    }
    pub fn push(&mut self, item: LiteGroup) {
        self.block.push(item)
    }
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

fn skip_whitespace(src: &mut Input) {
    while let Some((_, x)) = src.peek() {
        if x.is_whitespace() {
            let _ = src.next();
        } else {
            break;
        }
    }
}

#[derive(Clone, Copy)]
enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
}

impl From<BlockKind> for char {
    fn from(bk: BlockKind) -> char {
        match bk {
            BlockKind::Paren => ')',
            BlockKind::SquareBracket => ']',
            BlockKind::CurlyBracket => '}',
        }
    }
}

/// Finds the extents of a bare (un-classified) token, returning the string with its associated span,
/// along with any parse error that was discovered along the way.
/// Bare tokens are unparsed content separated by spaces or a command separator (like pipe or semicolon)
/// Bare tokens may be surrounded by quotes (single, double, or backtick) or braces (square, paren, curly)
pub fn bare(src: &mut Input, span_offset: usize) -> (Spanned<String>, Option<ParseError>) {
    let mut bare = String::new();
    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    let mut inside_quote: Option<char> = None;
    let mut block_level: Vec<BlockKind> = vec![];

    while let Some((_, c)) = src.peek() {
        let c = *c;
        if inside_quote.is_some() {
            if Some(c) == inside_quote {
                inside_quote = None;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            inside_quote = Some(c);
        } else if c == '[' {
            block_level.push(BlockKind::SquareBracket);
        } else if c == ']' {
            if let Some(BlockKind::SquareBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '{' {
            block_level.push(BlockKind::CurlyBracket);
        } else if c == '}' {
            if let Some(BlockKind::CurlyBracket) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if c == '(' {
            block_level.push(BlockKind::Paren);
        } else if c == ')' {
            if let Some(BlockKind::Paren) = block_level.last() {
                let _ = block_level.pop();
            }
        } else if block_level.is_empty() && (c.is_whitespace() || c == '|' || c == ';') {
            break;
        }
        bare.push(c);
        let _ = src.next();
    }

    let span = Span::new(
        start_offset + span_offset,
        start_offset + span_offset + bare.len(),
    );

    if let Some(block) = block_level.last() {
        let delim: char = (*block).into();
        let cause = ParseError::unexpected_eof(delim.to_string(), span);

        while let Some(bk) = block_level.pop() {
            bare.push(bk.into());
        }

        return (bare.spanned(span), Some(cause));
    }

    if let Some(delimiter) = inside_quote {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        bare.push(delimiter);

        return (
            bare.spanned(span),
            Some(ParseError::unexpected_eof(delimiter.to_string(), span)),
        );
    }

    if bare.is_empty() {
        return (
            bare.spanned(span),
            Some(ParseError::unexpected_eof("command".to_string(), span)),
        );
    }

    (bare.spanned(span), None)
}

/// Breaks the input string into a vector of tokens. This tokenization only tries to classify separators like
/// semicolons, pipes, etc from external bare values (values that haven't been classified further)
/// Takes in a string and and offset, which is used to offset the spans created (for when this function is used to parse inner strings)
pub fn lex(input: &str, span_offset: usize) -> (Vec<Token>, Option<ParseError>) {
    let mut char_indices = input.char_indices().peekable();
    let mut error = None;

    let mut output = vec![];

    while let Some((idx, c)) = char_indices.peek() {
        if *c == '|' {
            let idx = *idx;
            let _ = char_indices.next();
            output.push(Token::new(
                TokenContents::Pipe,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
        } else if *c == ';' {
            let idx = *idx;
            let _ = char_indices.next();
            output.push(Token::new(
                TokenContents::Semicolon,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
        } else if *c == '\n' || *c == '\r' {
            let idx = *idx;
            let _ = char_indices.next();
            output.push(Token::new(
                TokenContents::EOL,
                Span::new(span_offset + idx, span_offset + idx + 1),
            ));
        } else if *c == ' ' || *c == '\t' {
            let _ = char_indices.next();
        } else {
            let (result, err) = bare(&mut char_indices, span_offset);
            if error.is_none() {
                error = err;
            }
            let Spanned { item, span } = result;
            output.push(Token::new(TokenContents::Bare(item), span));
        }
    }

    (output, error)
}

// fn command(src: &mut Input, span_offset: usize) -> (LiteCommand, Option<ParseError>) {
//     let mut error = None;

//     let (v, err) = bare(src, span_offset);
//     let cmd = LiteCommand::new();
//     cmd.parts.push(v);
//     if error.is_none() {
//         error = err;
//     }

//     loop {
//         skip_whitespace(src);

//         if let Some((_, c)) = src.peek() {
//             // The first character tells us a lot about each argument
//             match c {
//                 ';' => {
//                     // this is the end of the command and the end of the pipeline
//                     break;
//                 }
//                 '|' => {
//                     let _ = src.next();
//                     if let Some((pos, next_c)) = src.peek() {
//                         if *next_c == '|' {
//                             // this isn't actually a pipeline but a comparison
//                             let span = Span::new(pos - 1 + span_offset, pos + 1 + span_offset);
//                             cmd.parts.push("||".to_string().spanned(span));
//                             let _ = src.next();
//                         } else {
//                             // this is the end of this command
//                             break;
//                         }
//                     } else {
//                         // this is the end of this command
//                         break;
//                     }
//                 }
//                 _ => {
//                     // basic argument
//                     let (v, err) = bare(src, span_offset);
//                     cmd.args.push(v);
//                     if error.is_none() {
//                         error = err;
//                     }
//                 }
//             }
//         } else {
//             break;
//         }
//     }

//     (cmd, None)
// }

// fn pipeline(src: &mut Input, span_offset: usize) -> (LiteGroup, Option<ParseError>) {
//     let mut pipelines = vec![];
//     let mut commands = vec![];
//     let mut error = None;

//     skip_whitespace(src);

//     while src.peek().is_some() {
//         // If there is content there, let's parse it
//         let (v, err) = command(src, span_offset);

//         if error.is_none() {
//             error = err;
//         }

//         commands.push(v);
//         skip_whitespace(src);

//         if let Some((_, ';')) = src.peek() {
//             let _ = src.next();

//             if !commands.is_empty() {
//                 pipelines.push(LitePipeline { commands });
//                 commands = vec![];
//             }
//         }
//     }

//     if !commands.is_empty() {
//         pipelines.push(LitePipeline { commands });
//     }

//     (LiteGroup { pipelines }, error)
// }

fn group(tokens: Vec<Token>) -> (LiteBlock, Option<ParseError>) {
    let mut groups = vec![];
    let mut group = LiteGroup::new();
    let mut pipeline = LitePipeline::new();
    let mut command = LiteCommand::new();

    for token in tokens {
        match token.contents {
            TokenContents::EOL => {
                if !command.is_empty() {
                    pipeline.push(command);
                    command = LiteCommand::new();
                }
                if !pipeline.is_empty() {
                    group.push(pipeline);
                    pipeline = LitePipeline::new();
                }
                if !group.is_empty() {
                    groups.push(group);
                    group = LiteGroup::new();
                }
            }
            TokenContents::Pipe => {
                if !command.is_empty() {
                    pipeline.push(command);
                    command = LiteCommand::new();
                } else {
                    let mut block = LiteBlock::new();
                    block.block = groups;

                    return (
                        block,
                        Some(ParseError::extra_tokens(
                            "|".to_string().spanned(token.span),
                        )),
                    );
                }
            }
            TokenContents::Semicolon => {
                if !command.is_empty() {
                    pipeline.push(command);
                    command = LiteCommand::new();
                }
                if !pipeline.is_empty() {
                    group.push(pipeline);
                    pipeline = LitePipeline::new();
                }
            }
            TokenContents::Bare(bare) => {
                command.push(bare.spanned(token.span));
            }
        }
    }
    if !command.is_empty() {
        pipeline.push(command);
    }
    if !pipeline.is_empty() {
        group.push(pipeline);
    }
    if !group.is_empty() {
        groups.push(group);
    }

    let mut block = LiteBlock::new();
    block.block = groups;
    (block, None)
}

pub fn lite_parse(src: &str, span_offset: usize) -> (LiteBlock, Option<ParseError>) {
    let (output, error) = lex(src, span_offset);
    if error.is_some() {
        return (LiteBlock::new(), error);
    }

    group(output)
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

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 3));
        }

        #[test]
        fn simple_2() {
            let input = "'foo bar' baz";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 9));
        }

        #[test]
        fn simple_3() {
            let input = "'foo\" bar' baz";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 10));
        }

        #[test]
        fn simple_4() {
            let input = "[foo bar] baz";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 9));
        }

        #[test]
        fn simple_5() {
            let input = "'foo 'bar baz";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 9));
        }

        #[test]
        fn simple_6() {
            let input = "''foo baz";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 5));
        }

        #[test]
        fn simple_7() {
            let input = "'' foo";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 2));
        }

        #[test]
        fn simple_8() {
            let input = " '' foo";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(1, 3));
        }

        #[test]
        fn simple_9() {
            let input = " 'foo' foo";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(1, 6));
        }

        #[test]
        fn simple_10() {
            let input = "[foo, bar]";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 10));
        }

        #[test]
        fn ignore_future() {
            let input = "foo 'bar";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_none());
            assert_eq!(result.span, span(0, 3));
        }

        #[test]
        fn invalid_1() {
            let input = "'foo bar";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_some());
        }

        #[test]
        fn invalid_2() {
            let input = "'bar";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_some());
        }

        #[test]
        fn invalid_4() {
            let input = " 'bar";

            let input = &mut input.char_indices().peekable();
            let (result, err) = bare(input, 0);

            assert!(err.is_some());
        }
    }

    mod lite_parse {
        use super::*;

        #[test]
        fn pipeline() {
            let (result, err) = lite_parse("cmd1 | cmd2 ; deploy", 0);
            assert!(err.is_none());
            assert_eq!(result.span(), span(0, 20));
            assert_eq!(result.block[0].span(), span(0, 11));
            assert_eq!(result.block[1].span(), span(14, 20));
        }

        #[test]
        fn simple_1() {
            let (result, err) = lite_parse("foo", 0);
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
            let (result, err) = lite_parse("foo", 10);
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
            let (result, err) = lite_parse("my_command \"foo' --test", 10);
            assert!(matches!(err.unwrap().reason(), nu_errors::ParseErrorReason::Eof { .. }));

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
    }
}
