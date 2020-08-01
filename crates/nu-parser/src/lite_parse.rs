use std::iter::Peekable;
use std::str::CharIndices;

use nu_source::{Span, Spanned, SpannedItem};

use crate::errors::{ParseError, ParseResult};

type Input<'t> = Peekable<CharIndices<'t>>;

#[derive(Debug, Clone)]
pub struct LiteCommand {
    pub name: Spanned<String>,
    pub args: Vec<Spanned<String>>,
}

impl LiteCommand {
    fn new(name: Spanned<String>) -> LiteCommand {
        LiteCommand { name, args: vec![] }
    }

    pub(crate) fn span(&self) -> Span {
        let start = self.name.span.start();
        let end = if let Some(x) = self.args.last() {
            x.span.end()
        } else {
            self.name.span.end()
        };

        Span::new(start, end)
    }
}

#[derive(Debug, Clone)]
pub struct LitePipeline {
    pub commands: Vec<LiteCommand>,
}

#[derive(Debug, Clone)]
pub struct LiteBlock {
    pub block: Vec<LitePipeline>,
}

impl From<Spanned<String>> for LiteCommand {
    fn from(v: Spanned<String>) -> LiteCommand {
        LiteCommand::new(v)
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

enum BlockKind {
    Paren,
    CurlyBracket,
    SquareBracket,
}

fn bare(src: &mut Input, span_offset: usize) -> ParseResult<Spanned<String>> {
    skip_whitespace(src);

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
        return Err(ParseError {
            cause: nu_errors::ParseError::unexpected_eof(
                match block {
                    BlockKind::Paren => ")",
                    BlockKind::SquareBracket => "]",
                    BlockKind::CurlyBracket => "}",
                },
                span,
            ),
            partial: Some(bare.spanned(span)),
        });
    }

    if let Some(delimiter) = inside_quote {
        // The non-lite parse trims quotes on both sides, so we add the expected quote so that
        // anyone wanting to consume this partial parse (e.g., completions) will be able to get
        // correct information from the non-lite parse.
        bare.push(delimiter);

        let span = Span::new(
            start_offset + span_offset,
            start_offset + span_offset + bare.len(),
        );

        return Err(ParseError {
            cause: nu_errors::ParseError::unexpected_eof(delimiter.to_string(), span),
            partial: Some(bare.spanned(span)),
        });
    }

    if bare.is_empty() {
        return Err(ParseError {
            cause: nu_errors::ParseError::unexpected_eof("command", span),
            partial: Some(bare.spanned(span)),
        });
    }

    Ok(bare.spanned(span))
}

fn command(src: &mut Input, span_offset: usize) -> ParseResult<LiteCommand> {
    let mut cmd = match bare(src, span_offset) {
        Ok(v) => LiteCommand::new(v),
        Err(e) => {
            return Err(ParseError {
                cause: e.cause,
                partial: e.partial.map(LiteCommand::new),
            });
        }
    };

    loop {
        skip_whitespace(src);

        if let Some((_, c)) = src.peek() {
            // The first character tells us a lot about each argument
            match c {
                ';' => {
                    // this is the end of the command and the end of the pipeline
                    break;
                }
                '|' => {
                    let _ = src.next();
                    if let Some((pos, next_c)) = src.peek() {
                        if *next_c == '|' {
                            // this isn't actually a pipeline but a comparison
                            let span = Span::new(pos - 1 + span_offset, pos + 1 + span_offset);
                            cmd.args.push("||".to_string().spanned(span));
                            let _ = src.next();
                        } else {
                            // this is the end of this command
                            break;
                        }
                    } else {
                        // this is the end of this command
                        break;
                    }
                }
                _ => {
                    // basic argument
                    match bare(src, span_offset) {
                        Ok(v) => {
                            cmd.args.push(v);
                        }

                        Err(e) => {
                            if let Some(v) = e.partial {
                                cmd.args.push(v);
                            }

                            return Err(ParseError {
                                cause: e.cause,
                                partial: Some(cmd),
                            });
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    Ok(cmd)
}

fn pipeline(src: &mut Input, span_offset: usize) -> ParseResult<LiteBlock> {
    let mut block = vec![];
    let mut commands = vec![];

    skip_whitespace(src);

    while src.peek().is_some() {
        // If there is content there, let's parse it
        let cmd = match command(src, span_offset) {
            Ok(v) => v,
            Err(e) => {
                if let Some(partial) = e.partial {
                    commands.push(partial);
                    block.push(LitePipeline { commands });
                }

                return Err(ParseError {
                    cause: e.cause,
                    partial: Some(LiteBlock { block }),
                });
            }
        };

        commands.push(cmd);
        skip_whitespace(src);

        if let Some((_, ';')) = src.peek() {
            let _ = src.next();

            if !commands.is_empty() {
                block.push(LitePipeline { commands });
                commands = vec![];
            }
        }
    }

    if !commands.is_empty() {
        block.push(LitePipeline { commands });
    }

    Ok(LiteBlock { block })
}

pub fn lite_parse(src: &str, span_offset: usize) -> ParseResult<LiteBlock> {
    pipeline(&mut src.char_indices().peekable(), span_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod bare {
        use super::*;

        #[test]
        fn simple_1() {
            let input = "foo bar baz";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 3);
        }

        #[test]
        fn simple_2() {
            let input = "'foo bar' baz";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 9);
        }

        #[test]
        fn simple_3() {
            let input = "'foo\" bar' baz";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 10);
        }

        #[test]
        fn simple_4() {
            let input = "[foo bar] baz";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 9);
        }

        #[test]
        fn simple_5() {
            let input = "'foo 'bar baz";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 9);
        }

        #[test]
        fn simple_6() {
            let input = "''foo baz";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 5);
        }

        #[test]
        fn simple_7() {
            let input = "'' foo";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 2);
        }

        #[test]
        fn simple_8() {
            let input = " '' foo";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 1);
            assert_eq!(result.span.end(), 3);
        }

        #[test]
        fn simple_9() {
            let input = " 'foo' foo";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 1);
            assert_eq!(result.span.end(), 6);
        }

        #[test]
        fn ignore_future() {
            let input = "foo 'bar";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0).unwrap();

            assert_eq!(result.span.start(), 0);
            assert_eq!(result.span.end(), 3);
        }

        #[test]
        fn invalid_1() {
            let input = "'foo bar";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0);

            assert_eq!(result.is_ok(), false);
        }

        #[test]
        fn invalid_2() {
            let input = "'bar";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0);

            assert_eq!(result.is_ok(), false);
        }

        #[test]
        fn invalid_4() {
            let input = " 'bar";

            let input = &mut input.char_indices().peekable();
            let result = bare(input, 0);

            assert_eq!(result.is_ok(), false);
        }
    }

    mod lite_parse {
        use super::*;

        #[test]
        fn simple_1() {
            let result = lite_parse("foo", 0).unwrap();
            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].commands.len(), 1);
            assert_eq!(result.block[0].commands[0].name.span.start(), 0);
            assert_eq!(result.block[0].commands[0].name.span.end(), 3);
        }

        #[test]
        fn simple_offset() {
            let result = lite_parse("foo", 10).unwrap();
            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].commands.len(), 1);
            assert_eq!(result.block[0].commands[0].name.span.start(), 10);
            assert_eq!(result.block[0].commands[0].name.span.end(), 13);
        }

        #[test]
        fn incomplete_result() {
            let result = lite_parse("my_command \"foo' --test", 10).unwrap_err();
            assert!(matches!(result.cause.reason(), nu_errors::ParseErrorReason::Eof { .. }));

            let result = result.partial.unwrap();
            assert_eq!(result.block.len(), 1);
            assert_eq!(result.block[0].commands.len(), 1);
            assert_eq!(result.block[0].commands[0].name.item, "my_command");
            assert_eq!(result.block[0].commands[0].args.len(), 1);
            assert_eq!(result.block[0].commands[0].args[0].item, "\"foo' --test\"");
        }
    }
}
