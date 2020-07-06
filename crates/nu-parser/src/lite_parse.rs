use std::iter::Peekable;
use std::str::CharIndices;

use nu_errors::ParseError;
use nu_source::{Span, Spanned, SpannedItem};

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

fn bare(src: &mut Input, span_offset: usize) -> Result<Spanned<String>, ParseError> {
    skip_whitespace(src);

    let mut bare = String::new();
    let start_offset = if let Some((pos, _)) = src.peek() {
        *pos
    } else {
        0
    };

    let mut escaping = false;
    let mut inside_quote: Option<char> = None;
    let mut block_level: Vec<BlockKind> = vec![];

    while let Some((_, c)) = src.peek() {
        let c = *c;

        if escaping {
            escaping = false;
        } else if c == '\\' {
            escaping = true;
            let _ = src.next();
            continue;
        } else if inside_quote.is_some() {
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
        return Err(ParseError::unexpected_eof(
            match block {
                BlockKind::Paren => ")",
                BlockKind::SquareBracket => "]",
                BlockKind::CurlyBracket => "}",
            },
            span,
        ));
    }

    if let Some(delimiter) = inside_quote {
        return Err(ParseError::unexpected_eof(delimiter.to_string(), span));
    }

    Ok(bare.spanned(span))
}

#[test]
fn bare_simple_1() -> Result<(), ParseError> {
    let input = "foo bar baz";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 3);

    Ok(())
}

#[test]
fn bare_simple_2() -> Result<(), ParseError> {
    let input = "'foo bar' baz";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 9);

    Ok(())
}

#[test]
fn bare_simple_3() -> Result<(), ParseError> {
    let input = "'foo\" bar' baz";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 10);

    Ok(())
}

#[test]
fn bare_simple_4() -> Result<(), ParseError> {
    let input = "[foo bar] baz";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 9);

    Ok(())
}

#[test]
fn bare_simple_5() -> Result<(), ParseError> {
    let input = "'foo 'bar baz";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 9);

    Ok(())
}

#[test]
fn bare_simple_6() -> Result<(), ParseError> {
    let input = "''foo baz";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 5);

    Ok(())
}

#[test]
fn bare_simple_7() -> Result<(), ParseError> {
    let input = "'' foo";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 2);

    Ok(())
}

#[test]
fn bare_simple_8() -> Result<(), ParseError> {
    let input = " '' foo";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 1);
    assert_eq!(result.span.end(), 3);

    Ok(())
}

#[test]
fn bare_simple_9() -> Result<(), ParseError> {
    let input = " 'foo' foo";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 1);
    assert_eq!(result.span.end(), 6);

    Ok(())
}

#[test]
fn bare_escape_1() -> Result<(), ParseError> {
    let input = " f\\'oo";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 1);
    assert_eq!(result.span.end(), 5);

    Ok(())
}

#[test]
fn bare_escape_2() -> Result<(), ParseError> {
    let input = " f\\\\'oo'";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 1);
    assert_eq!(result.span.end(), 7);

    Ok(())
}

#[test]
fn bare_ignore_future() -> Result<(), ParseError> {
    let input = "foo 'bar";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0)?;

    assert_eq!(result.span.start(), 0);
    assert_eq!(result.span.end(), 3);

    Ok(())
}

#[test]
fn bare_invalid_1() -> Result<(), ParseError> {
    let input = "'foo bar";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0);

    assert_eq!(result.is_ok(), false);

    Ok(())
}

#[test]
fn bare_invalid_2() -> Result<(), ParseError> {
    let input = "'bar";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0);

    assert_eq!(result.is_ok(), false);

    Ok(())
}

#[test]
fn bare_invalid_4() -> Result<(), ParseError> {
    let input = " 'bar";

    let input = &mut input.char_indices().peekable();
    let result = bare(input, 0);

    assert_eq!(result.is_ok(), false);

    Ok(())
}

fn command(src: &mut Input, span_offset: usize) -> Result<LiteCommand, ParseError> {
    let command = bare(src, span_offset)?;
    if command.item.is_empty() {
        Err(ParseError::unexpected_eof("command", command.span))
    } else {
        Ok(LiteCommand::new(command))
    }
}

fn pipeline(src: &mut Input, span_offset: usize) -> Result<LiteBlock, ParseError> {
    let mut block = vec![];
    let mut commands = vec![];

    skip_whitespace(src);

    while src.peek().is_some() {
        // If there is content there, let's parse it

        let mut cmd = match command(src, span_offset) {
            Ok(cmd) => cmd,
            Err(e) => return Err(e),
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
                        let arg = bare(src, span_offset)?;
                        cmd.args.push(arg);
                    }
                }
            } else {
                break;
            }
        }
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

pub fn lite_parse(src: &str, span_offset: usize) -> Result<LiteBlock, ParseError> {
    pipeline(&mut src.char_indices().peekable(), span_offset)
}

#[test]
fn lite_simple_1() -> Result<(), ParseError> {
    let result = lite_parse("foo", 0)?;
    assert_eq!(result.block.len(), 1);
    assert_eq!(result.block[0].commands.len(), 1);
    assert_eq!(result.block[0].commands[0].name.span.start(), 0);
    assert_eq!(result.block[0].commands[0].name.span.end(), 3);

    Ok(())
}

#[test]
fn lite_simple_offset() -> Result<(), ParseError> {
    let result = lite_parse("foo", 10)?;
    assert_eq!(result.block.len(), 1);
    assert_eq!(result.block[0].commands.len(), 1);
    assert_eq!(result.block[0].commands[0].name.span.start(), 10);
    assert_eq!(result.block[0].commands[0].name.span.end(), 13);

    Ok(())
}
